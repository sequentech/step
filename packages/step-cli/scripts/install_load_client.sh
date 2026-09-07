#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Sets up a plain Debian/Ubuntu machine (e.g. a cloud VM) to run the load-test
# scripts in this directory against a remote deployment — no devcontainer, no
# nix, no compose stack. Every step is idempotent: re-running after a failure
# only redoes what is missing, so a half-installed machine is never stuck.
#
#   install_load_client.sh [--with-provisioner] [--with-telephone] [--all]
#
# Default (no flags) installs what run_online_load_test.py needs on a load
# client: python3 + PyYAML, Node.js 20, the standalone Playwright package
# under packages/voting-portal/test/load, and the Chromium browser with its
# system libraries. That is enough to cast votes from a Stage-1 run dir
# copied from elsewhere (see docs/docusaurus/docs/07-developers/02-cli/
# 02-tutorials/load-testing/distributed-load-testing.md).
#
#   --with-provisioner  also installs the Rust toolchain and build deps and
#                       builds step-cli (needed only on the ONE machine that
#                       runs setup_telephone_load_test.py / cleanup).
#   --with-telephone    also initialises the beyond submodule (needs git
#                       access to it), builds ivr-cli, and installs a local
#                       redis-server for run_telephone_load_test.py (set
#                       telephone_run.valkey_url: redis://127.0.0.1:6379).
#                       Implies --with-provisioner's Rust toolchain.
#   --all               both of the above.
#
# Must run as root or as a user with passwordless sudo for the apt-get
# calls (run `sudo -v` first on a machine that prompts for the password).
# Set SKIP_APT=1 on a machine without sudo where the system packages are
# already in place: apt-get is then never called, and a missing Node.js is
# installed user-locally under ~/.local instead of via NodeSource.

set -euo pipefail

SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPTS_DIR/../../.." && pwd)"
LOAD_TEST_DIR="$REPO_ROOT/packages/voting-portal/test/load"
STEP_CLI_TARGET="$REPO_ROOT/packages/step-cli/rust-local-target"
IVR_CLI_TARGET="$REPO_ROOT/beyond/packages/rust-local-target"
RUST_TOOLCHAIN=1.96.0
NODE_MAJOR=20

WITH_PROVISIONER=0
WITH_TELEPHONE=0
for arg in "$@"; do
  case "$arg" in
    --with-provisioner) WITH_PROVISIONER=1 ;;
    --with-telephone) WITH_TELEPHONE=1 ;;
    --all) WITH_PROVISIONER=1; WITH_TELEPHONE=1 ;;
    -h|--help) sed -n '6,34p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown flag: $arg (see --help)" >&2; exit 1 ;;
  esac
done

log() { echo "==> $*" >&2; }

SUDO=""
if [ "$(id -u)" -ne 0 ]; then
  SUDO="sudo"
fi
if [ "${SKIP_APT:-0}" != "1" ] && [ -n "$SUDO" ] && ! sudo -n true 2>/dev/null; then
  cat >&2 <<EOF
This script installs system packages with apt-get and needs sudo without a
password prompt. Either:
  - run 'sudo -v' first (caches your password for a few minutes), then re-run, or
  - run with SKIP_APT=1 if the system packages are already installed
    (Node.js is then installed under ~/.local when missing; Playwright's
    Chromium system libraries must already be present).
EOF
  exit 1
fi

apt_install() {
  if [ "${SKIP_APT:-0}" = "1" ]; then
    log "SKIP_APT=1: not installing system packages: $*"
    return
  fi
  local missing=()
  for pkg in "$@"; do
    if ! dpkg-query -W -f='${Status}' "$pkg" 2>/dev/null | grep -q "install ok installed"; then
      missing+=("$pkg")
    fi
  done
  if [ "${#missing[@]}" -eq 0 ]; then
    log "System packages already installed: $*"
    return
  fi
  log "Installing system packages: ${missing[*]}"
  $SUDO apt-get update -qq
  DEBIAN_FRONTEND=noninteractive $SUDO apt-get install -y -qq --no-install-recommends "${missing[@]}"
}

# --- Base: python3 + PyYAML, Node.js, Playwright + Chromium -------------------

apt_install ca-certificates curl git rsync python3 python3-yaml

# User-local Node.js (no root needed): the official tarball unpacked under
# ~/.local/node-v<major>, with node/npm/npx linked into ~/.local/bin — which
# Ubuntu's default ~/.profile puts on PATH at login when it exists.
install_node_userlocal() {
  local version prefix
  version=$(curl -fsSL https://nodejs.org/dist/index.json \
    | python3 -c "import json,sys; print(next(v['version'] for v in json.load(sys.stdin) if v['version'].startswith('v$NODE_MAJOR.')))")
  prefix="$HOME/.local/node-v$NODE_MAJOR"
  log "Installing Node.js $version under $prefix (no sudo)"
  mkdir -p "$prefix" "$HOME/.local/bin"
  curl -fsSL "https://nodejs.org/dist/$version/node-$version-linux-x64.tar.xz" | tar -xJ --strip-components=1 -C "$prefix"
  ln -sf "$prefix/bin/node" "$prefix/bin/npm" "$prefix/bin/npx" "$HOME/.local/bin/"
  export PATH="$HOME/.local/bin:$PATH"
  NODE_PATH_HINT="$HOME/.local/bin"
}

NODE_PATH_HINT=""
if [ -x "$HOME/.local/bin/node" ]; then
  export PATH="$HOME/.local/bin:$PATH"
fi
if ! command -v node >/dev/null 2>&1 || [ "$(node -p 'process.versions.node.split(".")[0]')" -lt "$NODE_MAJOR" ]; then
  if [ "${SKIP_APT:-0}" = "1" ]; then
    install_node_userlocal
  else
    log "Installing Node.js $NODE_MAJOR (NodeSource)"
    curl -fsSL "https://deb.nodesource.com/setup_${NODE_MAJOR}.x" | $SUDO bash -
    $SUDO apt-get install -y -qq nodejs
  fi
fi
log "Node.js $(node --version) at $(command -v node)"

if [ -x "$LOAD_TEST_DIR/node_modules/.bin/playwright" ]; then
  log "Playwright package already installed under $LOAD_TEST_DIR"
else
  log "Installing the standalone Playwright package under $LOAD_TEST_DIR"
  (cd "$LOAD_TEST_DIR" && npm install --no-audit --no-fund --no-package-lock)
fi

# `playwright install` is itself idempotent (a browser already present at the
# pinned revision is a no-op); --with-deps adds the Chromium shared libraries
# via apt, which is the part a bare VM is missing.
PW_BROWSERS_PATH="${PLAYWRIGHT_BROWSERS_PATH:-$HOME/.cache/ms-playwright}"
if compgen -G "$PW_BROWSERS_PATH/chromium*" >/dev/null && [ "${SKIP_APT:-0}" = "1" ]; then
  log "Playwright Chromium already present under $PW_BROWSERS_PATH"
else
  log "Installing Playwright Chromium (and its system libraries)"
  if [ "${SKIP_APT:-0}" = "1" ]; then
    (cd "$LOAD_TEST_DIR" && npx playwright install chromium)
  else
    (cd "$LOAD_TEST_DIR" && npx playwright install --with-deps chromium)
  fi
fi

# --- Optional: Rust toolchain + step-cli (provisioner machine only) ----------

install_rust() {
  # Same package list packages/loadtesting/Dockerfile and the step-cli CI
  # workflow build against.
  apt_install build-essential pkg-config libssl-dev protobuf-compiler libprotobuf-dev m4
  if [ ! -x "$HOME/.cargo/bin/cargo" ]; then
    log "Installing Rust $RUST_TOOLCHAIN (rustup)"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --default-toolchain "$RUST_TOOLCHAIN" --profile minimal
  fi
  export PATH="$HOME/.cargo/bin:$PATH"
  if ! rustup toolchain list | grep -q "^$RUST_TOOLCHAIN"; then
    rustup toolchain install "$RUST_TOOLCHAIN" --profile minimal
  fi
  log "Rust: $(rustc +"$RUST_TOOLCHAIN" --version)"
}

if [ "$WITH_PROVISIONER" = "1" ] || [ "$WITH_TELEPHONE" = "1" ]; then
  install_rust
fi

if [ "$WITH_PROVISIONER" = "1" ]; then
  if [ -x "$STEP_CLI_TARGET/release/step-cli" ]; then
    log "step-cli already built at $STEP_CLI_TARGET/release/step-cli (delete it to force a rebuild)"
  else
    log "Building step-cli (release) — this pulls in sequent-core and windmill, expect a long first build"
    (cd "$REPO_ROOT/packages" && CARGO_TARGET_DIR="$STEP_CLI_TARGET" cargo +"$RUST_TOOLCHAIN" build --release -p step-cli)
  fi
fi

# --- Optional: ivr-cli + local redis (telephone load clients) ----------------

if [ "$WITH_TELEPHONE" = "1" ]; then
  if [ ! -f "$REPO_ROOT/beyond/packages/Cargo.toml" ]; then
    log "Initialising the beyond submodule (private repository — git access required)"
    (cd "$REPO_ROOT" && git submodule update --init --depth 1 beyond)
  fi
  if [ -x "$IVR_CLI_TARGET/release/ivr-cli" ]; then
    log "ivr-cli already built at $IVR_CLI_TARGET/release/ivr-cli (delete it to force a rebuild)"
  else
    log "Building ivr-cli (release)"
    (cd "$REPO_ROOT/beyond/packages" && CARGO_TARGET_DIR="$IVR_CLI_TARGET" cargo +"$RUST_TOOLCHAIN" build --release -p ivr-cli)
  fi
  apt_install redis-server
  if [ "${SKIP_APT:-0}" != "1" ]; then
    $SUDO systemctl enable --now redis-server >/dev/null 2>&1 || log "WARNING: could not start redis-server via systemd — start a Redis/Valkey yourself and set telephone_run.valkey_url"
  fi
fi

# --- Summary -------------------------------------------------------------------

CONFIG_DIR="$SCRIPTS_DIR/telephone-load-test-inputs/config"
if [ ! -f "$CONFIG_DIR/layers.yaml" ]; then
  mkdir -p "$CONFIG_DIR"
  cp "$SCRIPTS_DIR/telephone-load-test-inputs/layers.yaml.example" "$CONFIG_DIR/layers.yaml"
  log "Created $CONFIG_DIR/layers.yaml from the tracked template — fill in the target server's URLs and credentials before running anything"
fi

log "Done. Next steps:"
if [ -n "$NODE_PATH_HINT" ]; then
  echo "  export PATH=\"$NODE_PATH_HINT:\$PATH\"   # user-local Node.js (already on PATH in new login shells)" >&2
fi
if [ "$WITH_PROVISIONER" = "1" ]; then
  echo "  export PATH=\"$STEP_CLI_TARGET/release:\$PATH\"" >&2
fi
if [ "$WITH_TELEPHONE" = "1" ]; then
  echo "  set telephone_run.valkey_url: redis://127.0.0.1:6379 in $CONFIG_DIR/layers.yaml" >&2
fi
echo "  edit $CONFIG_DIR/layers.yaml, then: python3 $SCRIPTS_DIR/run_online_load_test.py" >&2
