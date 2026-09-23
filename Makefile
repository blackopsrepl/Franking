# ╔══════════════════════════════════════════════════════════════════════════════╗
# ║                             FRANKING                                       ║
# ║                 ratatui TUI email client · app-owned mail engine           ║
# ╚══════════════════════════════════════════════════════════════════════════════╝
#
# Part of SolverForge Linux — https://solverforge.com
#

SHELL := /bin/bash

# ── Colors & Symbols ─────────────────────────────────────────────────────────

GREEN    := \033[92m
CYAN     := \033[96m
YELLOW   := \033[93m
MAGENTA  := \033[95m
RED      := \033[91m
GRAY     := \033[90m
BOLD     := \033[1m
RESET    := \033[0m

CHECK    := ✓
CROSS    := ✗
ARROW    := ▸
PROGRESS := →

# Printed after a release: the tag and the branch go to every remote.
define release_pushed
printf "\n$(GREEN)$(CHECK) Release committed and tagged. Publish with:$(RESET)\n"; \
printf "    git push blackopsrepl main:main --follow-tags\n"; \
printf "    git push vigilance main:main --follow-tags\n\n"
endef

# ── Project Metadata ─────────────────────────────────────────────────────────

NAME     := franking
CARGO_HOME ?= $(HOME)/.cargo
VERSION  := $(shell grep -m1 '^version' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
BIN      := target/release/$(NAME)
BIN_DBG  := target/debug/$(NAME)

# ── Install Paths (SolverForge Linux framework) ───────────────────────────────

SF_HOME  := $(HOME)/.local/share/solverforge
SF_SHARE := $(SF_HOME)/mail

# ── Phony Targets ─────────────────────────────────────────────────────────────

.PHONY: help build release debug check clippy fmt fmt-check test live-test lint ci pre-release version
.PHONY: dev run run-account
.PHONY: install uninstall setup accounts
.PHONY: bump-dry bump-patch bump-minor bump-major release-tool
.PHONY: clean dist-clean loc info deps-check

.DEFAULT_GOAL := help

# ── Banner ────────────────────────────────────────────────────────────────────

define banner
	@printf "$(CYAN)$(BOLD)╔══════════════════════════════════════╗$(RESET)\n"
	@printf "$(CYAN)$(BOLD)║  ✉  franking %-15s ║$(RESET)\n" "v$(VERSION)"
	@printf "$(CYAN)$(BOLD)╚══════════════════════════════════════╝$(RESET)\n\n"
endef

# ══════════════════════════════════════════════════════════════════════════════
#  BUILD
# ══════════════════════════════════════════════════════════════════════════════

release: deps-check ## Build optimized release binary
	$(call banner)
	@printf "$(CYAN)$(BOLD)╔══════════════════════════════════════╗$(RESET)\n"
	@printf "$(CYAN)$(BOLD)║          Release Build               ║$(RESET)\n"
	@printf "$(CYAN)$(BOLD)╚══════════════════════════════════════╝$(RESET)\n\n"
	@printf "$(PROGRESS) Building release binary...\n"
	@set -o pipefail; cargo build --locked --release 2>&1 | sed 's/^/    /' && \
		printf "$(GREEN)$(CHECK) Release build successful$(RESET)\n\n" || \
		(printf "$(RED)$(CROSS) Release build failed$(RESET)\n\n" && exit 1)

build: release ## Alias for release

debug: ## Build debug binary
	@printf "$(PROGRESS) Building debug binary...\n"
	@set -o pipefail; cargo build --locked 2>&1 | sed 's/^/    /' && \
		printf "$(GREEN)$(CHECK) Debug build successful$(RESET)\n" || \
		(printf "$(RED)$(CROSS) Debug build failed$(RESET)\n" && exit 1)

check: ## Type-check without codegen (fast)
	@printf "$(PROGRESS) Type checking...\n"
	@set -o pipefail; cargo check --locked 2>&1 | sed 's/^/    /' && \
		printf "$(GREEN)$(CHECK) Type check passed$(RESET)\n" || \
		(printf "$(RED)$(CROSS) Type check failed$(RESET)\n" && exit 1)

# ══════════════════════════════════════════════════════════════════════════════
#  QUALITY
# ══════════════════════════════════════════════════════════════════════════════

clippy: ## Run clippy lints
	@printf "$(PROGRESS) Running clippy...\n"
	@set -o pipefail; cargo clippy --locked --all-targets -- -D warnings 2>&1 | sed 's/^/    /' && \
		printf "$(GREEN)$(CHECK) Clippy passed$(RESET)\n" || \
		(printf "$(RED)$(CROSS) Clippy warnings found$(RESET)\n" && exit 1)

fmt: ## Format code with rustfmt
	@printf "$(PROGRESS) Formatting code...\n"
	@cargo fmt --all
	@printf "$(GREEN)$(CHECK) Code formatted$(RESET)\n"

fmt-check: ## Check formatting without changes
	@printf "$(PROGRESS) Checking formatting...\n"
	@cargo fmt --all -- --check && \
		printf "$(GREEN)$(CHECK) Formatting valid$(RESET)\n" || \
		(printf "$(RED)$(CROSS) Formatting issues found$(RESET)\n" && exit 1)

test: ## Run all tests
	@printf "$(CYAN)$(BOLD)╔══════════════════════════════════════╗$(RESET)\n"
	@printf "$(CYAN)$(BOLD)║           Full Test Suite            ║$(RESET)\n"
	@printf "$(CYAN)$(BOLD)╚══════════════════════════════════════╝$(RESET)\n\n"
	@printf "$(PROGRESS) Running all tests...\n"
	@set -o pipefail; cargo test --locked 2>&1 | sed 's/^/    /' && \
		printf "\n$(GREEN)$(CHECK) All tests passed$(RESET)\n\n" || \
		(printf "\n$(RED)$(CROSS) Tests failed$(RESET)\n\n" && exit 1)

live-test: ## Run the live IMAP/SMTP integration tests in containers
	@bash scripts/live-test.sh

lint: fmt-check clippy ## Run all lints (fmt-check + clippy)
	@printf "\n$(GREEN)$(BOLD)$(CHECK) All lint checks passed$(RESET)\n\n"

ci: lint test release ## Local CI-style validation: lint → test → build
	@printf "$(GREEN)$(BOLD)╔══════════════════════════════════════╗$(RESET)\n"
	@printf "$(GREEN)$(BOLD)║   $(CHECK) Local validation passed           ║$(RESET)\n"
	@printf "$(GREEN)$(BOLD)╚══════════════════════════════════════╝$(RESET)\n\n"

pre-release: lint test release ## Run release-oriented validation
	@printf "$(GREEN)$(BOLD)╔══════════════════════════════════════╗$(RESET)\n"
	@printf "$(GREEN)$(BOLD)║   $(CHECK) Pre-release checks passed        ║$(RESET)\n"
	@printf "$(GREEN)$(BOLD)╚══════════════════════════════════════╝$(RESET)\n"
	@printf "$(GREEN)$(BOLD)Ready for release: v$(VERSION)$(RESET)\n\n"

# ══════════════════════════════════════════════════════════════════════════════
#  RUN
# ══════════════════════════════════════════════════════════════════════════════

dev: debug ## Build debug and run
	@printf "$(ARROW) Running in debug mode...\n"
	@./$(BIN_DBG)

run: release ## Build release and run
	@printf "$(ARROW) Running in release mode...\n"
	@./$(BIN)

run-account: release ## Run with specific account (ACCOUNT=name)
	@printf "$(ARROW) Running with account: $(YELLOW)$(ACCOUNT)$(RESET)\n"
	@./$(BIN) --account $(ACCOUNT)

# ══════════════════════════════════════════════════════════════════════════════
#  INSTALL  (binary into CARGO_HOME, helper into the SolverForge Linux share)
# ══════════════════════════════════════════════════════════════════════════════

install: ## Install the binary and the SolverForge Linux setup helper
	$(call banner)
	@printf "$(CYAN)$(BOLD)╔══════════════════════════════════════╗$(RESET)\n"
	@printf "$(CYAN)$(BOLD)║    Installing $(NAME)$(RESET)\n"
	@printf "$(CYAN)$(BOLD)╚══════════════════════════════════════╝$(RESET)\n\n"
	@printf "$(PROGRESS) Installing binary → $(CARGO_HOME)/bin/$(NAME)\n"
	@cargo install --path . --locked --force
	@printf "$(GREEN)$(CHECK) Binary installed$(RESET)\n"
	@printf "$(PROGRESS) Installing setup scripts → $(SF_SHARE)/\n"
	@install -d $(SF_SHARE)
	@install -m755 setup-accounts.sh    $(SF_SHARE)/
	@printf "$(GREEN)$(CHECK) Setup scripts installed$(RESET)\n"
	@printf "\n$(GREEN)$(BOLD)$(CHECK) Installed $(NAME) v$(VERSION)$(RESET)\n\n"

uninstall: ## Remove the installed binary and setup helper
	@printf "$(PROGRESS) Removing $(CARGO_HOME)/bin/$(NAME)...\n"
	@cargo uninstall $(NAME) || true
	@printf "$(PROGRESS) Removing $(SF_SHARE)/...\n"
	@rm -rf $(SF_SHARE)
	@printf "$(GREEN)$(CHECK) Uninstalled$(RESET)\n"

# ══════════════════════════════════════════════════════════════════════════════
#  SETUP
# ══════════════════════════════════════════════════════════════════════════════

setup: release ## Interactive account setup wizard
	@printf "$(ARROW) Launching account setup wizard...\n"
	@./$(BIN) --setup

accounts: release ## List configured email accounts and their status
	@printf "$(PROGRESS) Querying Franking runtime...\n"
	@./$(BIN) --accounts

# ══════════════════════════════════════════════════════════════════════════════
#  HOUSEKEEPING
# ══════════════════════════════════════════════════════════════════════════════

clean: ## Remove build artifacts
	@printf "$(PROGRESS) Cleaning target/...\n"
	@cargo clean
	@printf "$(GREEN)$(CHECK) Clean complete$(RESET)\n"

dist-clean: clean ## Clean everything including Cargo.lock
	@printf "$(PROGRESS) Removing Cargo.lock...\n"
	@rm -f Cargo.lock
	@printf "$(GREEN)$(CHECK) Dist-clean complete$(RESET)\n"

# ══════════════════════════════════════════════════════════════════════════════
#  INFO
# ══════════════════════════════════════════════════════════════════════════════

deps-check: ## Verify build dependencies
	@command -v cargo >/dev/null 2>&1 || \
		(printf "$(RED)$(CROSS) cargo not found — install Rust via https://rustup.rs$(RESET)\n" && exit 1)

loc: ## Count lines of code
	$(call banner)
	@printf "  $(GRAY)%-30s %s$(RESET)\n" 'File' 'Lines'
	@printf "  $(GRAY)%-30s %s$(RESET)\n" '──────────────────────────────' '─────'
	@find src -name '*.rs' | sort | while read f; do \
		printf "  %-30s %s\n" "$$f" "$$(wc -l < "$$f")"; \
	done
	@printf "  $(GRAY)%-30s %s$(RESET)\n" '──────────────────────────────' '─────'
	@printf "  $(BOLD)%-30s %s$(RESET)\n" 'Total' "$$(find src -name '*.rs' -exec cat {} + | wc -l)"

info: ## Show project info
	$(call banner)
	@printf "  $(GRAY)version$(RESET)    %s\n" "$(VERSION)"
	@printf "  $(GRAY)rustc$(RESET)      %s\n" "$$(rustc --version 2>/dev/null || echo 'not found')"
	@printf "  $(GRAY)cargo$(RESET)      %s\n" "$$(cargo --version 2>/dev/null || echo 'not found')"
	@printf "  $(GRAY)binary$(RESET)     %s\n" "$(BIN)"
	@printf "  $(GRAY)install→$(RESET)   %s\n" "$(CARGO_HOME)/bin/$(NAME)"
	@echo

version: ## Print the current crate version
	@printf "$(YELLOW)$(BOLD)%s$(RESET)\n" "$(VERSION)"

# ══════════════════════════════════════════════════════════════════════════════
#  RELEASE  (commit-and-tag-version owns the version surfaces and changelog)
# ══════════════════════════════════════════════════════════════════════════════

release-tool:
	@command -v commit-and-tag-version >/dev/null 2>&1 || \
		(printf "$(RED)$(CROSS) commit-and-tag-version not found — install it with 'npm i -g commit-and-tag-version'$(RESET)\n" && exit 1)

bump-dry: release-tool ## Preview the next version and changelog without writing
	@printf "$(PROGRESS) Previewing the next release...\n\n"
	@commit-and-tag-version --dry-run

bump-patch: release-tool pre-release ## Release a patch (x.y.Z)
	@commit-and-tag-version --release-as patch
	@$(call release_pushed)

bump-minor: release-tool pre-release ## Release a minor (x.Y.0)
	@commit-and-tag-version --release-as minor
	@$(call release_pushed)

bump-major: release-tool pre-release ## Release a major (X.0.0)
	@commit-and-tag-version --release-as major
	@$(call release_pushed)

# ══════════════════════════════════════════════════════════════════════════════
#  HELP
# ══════════════════════════════════════════════════════════════════════════════

help:
	$(call banner)
	@/bin/echo -e "$(CYAN)$(BOLD)Build:$(RESET)"
	@/bin/echo -e "  $(GREEN)make$(RESET)                  - Show this help"
	@/bin/echo -e "  $(GREEN)make release$(RESET)          - Build optimized release binary"
	@/bin/echo -e "  $(GREEN)make debug$(RESET)            - Build debug binary"
	@/bin/echo -e "  $(GREEN)make check$(RESET)            - Type-check (fast)"
	@/bin/echo -e ""
	@/bin/echo -e "$(CYAN)$(BOLD)Quality:$(RESET)"
	@/bin/echo -e "  $(GREEN)make test$(RESET)             - Run all tests"
	@/bin/echo -e "  $(GREEN)make live-test$(RESET)        - Run live tests against Dovecot and Mailpit"
	@/bin/echo -e "  $(GREEN)make lint$(RESET)             - fmt-check + clippy"
	@/bin/echo -e "  $(GREEN)make fmt$(RESET)              - Format code"
	@/bin/echo -e "  $(GREEN)make clippy$(RESET)           - Run clippy lints"
	@/bin/echo -e "  $(GREEN)make ci$(RESET)               - $(YELLOW)$(BOLD)Local CI-style validation: lint → test → build$(RESET)"
	@/bin/echo -e "  $(GREEN)make pre-release$(RESET)      - Release-oriented validation"
	@/bin/echo -e ""
	@/bin/echo -e "$(CYAN)$(BOLD)Release:$(RESET)"
	@/bin/echo -e "  $(GREEN)make bump-dry$(RESET)         - Preview the next version and changelog"
	@/bin/echo -e "  $(GREEN)make bump-patch$(RESET)       - Release a patch (x.y.Z)"
	@/bin/echo -e "  $(GREEN)make bump-minor$(RESET)       - Release a minor (x.Y.0)"
	@/bin/echo -e "  $(GREEN)make bump-major$(RESET)       - Release a major (X.0.0)"
	@/bin/echo -e ""
	@/bin/echo -e "$(CYAN)$(BOLD)Run:$(RESET)"
	@/bin/echo -e "  $(GREEN)make dev$(RESET)              - Build debug + run"
	@/bin/echo -e "  $(GREEN)make run$(RESET)              - Build release + run"
	@/bin/echo -e "  $(GREEN)make run-account ACCOUNT=x$(RESET) - Run with specific account"
	@/bin/echo -e ""
	@/bin/echo -e "$(CYAN)$(BOLD)Install (SolverForge Linux):$(RESET)"
	@/bin/echo -e "  $(GREEN)make install$(RESET)          - Install the binary and the setup helper"
	@/bin/echo -e "  $(GREEN)make uninstall$(RESET)        - Remove from SolverForge"
	@/bin/echo -e "  $(GREEN)make setup$(RESET)            - Interactive account wizard"
	@/bin/echo -e "  $(GREEN)make accounts$(RESET)         - List configured accounts"
	@/bin/echo -e ""
	@/bin/echo -e "$(CYAN)$(BOLD)Other:$(RESET)"
	@/bin/echo -e "  $(GREEN)make clean$(RESET)            - Remove build artifacts"
	@/bin/echo -e "  $(GREEN)make info$(RESET)             - Show project info"
	@/bin/echo -e "  $(GREEN)make loc$(RESET)              - Count lines of code"
	@/bin/echo -e "  $(GREEN)make version$(RESET)          - Print crate version"
	@/bin/echo -e ""
	@/bin/echo -e "$(GRAY)Current version: v$(VERSION)$(RESET)"
	@/bin/echo -e ""
