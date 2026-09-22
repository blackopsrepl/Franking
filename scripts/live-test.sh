#!/usr/bin/env bash
# Bring up the live mail services and run the integration tests against them.
#
# Dovecot serves IMAP and ManageSieve (Pigeonhole); Mailpit is the SMTP sink for
# the send path. Both run as throwaway containers that are removed on exit.
#
# Usage:
#   scripts/live-test.sh                 # run the live test targets
#   scripts/live-test.sh --help          # list the test targets
#   KEEP=1 scripts/live-test.sh          # leave the containers running
#   IMAP_PORT=2153 scripts/live-test.sh  # choose the host ports
#
# Environment:
#   IMAP_PORT   host port for IMAP           (default 1153)
#   SIEVE_PORT  host port for ManageSieve    (default 4190)
#   SMTP_PORT   host port for SMTP           (default 1025)
#   KEEP        set to 1 to keep containers after the run
set -euo pipefail

IMAP_PORT=${IMAP_PORT:-1153}
SIEVE_PORT=${SIEVE_PORT:-4190}
SMTP_PORT=${SMTP_PORT:-1025}
DOVECOT_IMAGE=${DOVECOT_IMAGE:-dovecot/dovecot:latest}
MAILPIT_IMAGE=${MAILPIT_IMAGE:-axllent/mailpit:latest}
DOVECOT_CONTAINER=${DOVECOT_CONTAINER:-franking-dovecot}
MAILPIT_CONTAINER=${MAILPIT_CONTAINER:-franking-mailpit}
KEEP=${KEEP:-0}

DOVECOT_USER='test'
DOVECOT_PASSWORD='password'

if [[ ${1:-} == --help ]]; then
  sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'
  exit 0
fi

if ! command -v podman >/dev/null; then
  printf 'podman is required for the live test services\n' >&2
  exit 1
fi

state_dir=$(mktemp -d)
cleanup() {
  if [[ $KEEP == 1 ]]; then
    printf '\nkept: %s (IMAP %s, ManageSieve %s) and %s (SMTP %s)\n' \
      "$DOVECOT_CONTAINER" "$IMAP_PORT" "$SIEVE_PORT" "$MAILPIT_CONTAINER" "$SMTP_PORT"
  else
    podman rm -f "$DOVECOT_CONTAINER" "$MAILPIT_CONTAINER" >/dev/null 2>&1 || true
  fi
  rm -rf "$state_dir"
}
trap cleanup EXIT

# Dovecot refuses cleartext authentication on an unencrypted listener unless the
# drop-in allows it, and the rootless image publishes IMAP on 31143 with
# ManageSieve on 34190.
printf 'auth_allow_cleartext = yes\n' >"$state_dir/99-test.conf"

container_exists() {
  podman container exists "$1" >/dev/null 2>&1
}

wait_for_port() {
  local port=$1 name=$2 attempts=0
  while (( attempts < 100 )); do
    if (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null; then
      exec 3<&- 3>&-
      return 0
    fi
    attempts=$((attempts + 1))
    sleep 0.2
  done
  printf '%s did not open port %s\n' "$name" "$port" >&2
  return 1
}

start_dovecot() {
  if container_exists "$DOVECOT_CONTAINER"; then
    podman rm -f "$DOVECOT_CONTAINER" >/dev/null
  fi
  podman run -d --name "$DOVECOT_CONTAINER" \
    -p "$IMAP_PORT":31143 \
    -p "$SIEVE_PORT":34190 \
    -e USER_PASSWORD="$DOVECOT_PASSWORD" \
    -v "$state_dir/99-test.conf":/etc/dovecot/conf.d/99-test.conf:Z \
    "$DOVECOT_IMAGE" >/dev/null
  wait_for_port "$IMAP_PORT" Dovecot
}

start_mailpit() {
  if container_exists "$MAILPIT_CONTAINER"; then
    podman rm -f "$MAILPIT_CONTAINER" >/dev/null
  fi
  podman run -d --name "$MAILPIT_CONTAINER" \
    -p "$SMTP_PORT":1025 \
    "$MAILPIT_IMAGE" >/dev/null
  wait_for_port "$SMTP_PORT" Mailpit
}

printf 'starting Dovecot (IMAP %s, ManageSieve %s)...\n' "$IMAP_PORT" "$SIEVE_PORT"
start_dovecot
printf 'starting Mailpit (SMTP %s)...\n' "$SMTP_PORT"
start_mailpit

# The ManageSieve listener is part of the same Dovecot service; report its state
# rather than assuming, since the image maps it to its own internal port.
if wait_for_port "$SIEVE_PORT" "Dovecot ManageSieve"; then
  printf 'ManageSieve is accepting connections\n'
else
  printf 'warning: ManageSieve did not come up; Sieve tests will be skipped\n' >&2
fi

export FRANKING_IMAP_TEST_ADDR="127.0.0.1:$IMAP_PORT"
export FRANKING_IMAP_TEST_USER="$DOVECOT_USER"
export FRANKING_IMAP_TEST_PASSWORD="$DOVECOT_PASSWORD"
export FRANKING_SIEVE_TEST_ADDR="127.0.0.1:$SIEVE_PORT"
export FRANKING_SMTP_TEST_ADDR="127.0.0.1:$SMTP_PORT"

targets=(--test imap_codec --test dovecot_test --test sieve_live_test)
printf '\nrunning: cargo test %s\n\n' "${targets[*]}"
cargo test "${targets[@]}" "$@"
