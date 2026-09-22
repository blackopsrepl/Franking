#!/usr/bin/env bash
# Fail if any tracked Rust source file reaches the repository line limit.
set -euo pipefail

limit=300
status=0

while IFS= read -r file; do
  [[ -f "$file" ]] || continue
  lines=$(wc -l < "$file")
  if (( lines >= limit )); then
    printf '%s:%s exceeds %s lines\n' "$file" "$lines" "$limit" >&2
    status=1
  fi
done < <(git ls-files '*.rs')

exit "$status"
