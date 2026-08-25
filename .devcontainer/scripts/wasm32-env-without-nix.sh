#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# The wasm32 build environment, for a machine without Nix.
#
# CI and the devcontainer build the WASM through `nix develop`, and the flake in
# `packages/sequent-core` sets up a wasm-capable clang for it. On a plain macOS
# checkout there is no Nix and Apple's own clang has the WebAssembly backend
# disabled — `clang --print-targets` does not list wasm32 — so `ring`'s build
# script fails with:
#
#     unable to create target: 'No available targets are compatible with
#     triple "wasm32-unknown-unknown"'
#
# which reads like a Rust problem and is not one. `rustup target add
# wasm32-unknown-unknown` does not help either: the missing piece is a C
# compiler, because `ring` still compiles C for that target.
#
# So this reproduces what the flake exports, from Homebrew's LLVM instead of
# Nix's. Source it, do not run it:
#
#     brew install llvm            # once
#     source .devcontainer/scripts/wasm32-env-without-nix.sh
#     cargo check -p sequent-core --features wasmtest,default_features,\
#         election_config_xlsx,election_config_templates,election_config_archive \
#         --target wasm32-unknown-unknown
#
# `nix develop` remains the supported path and the one CI uses. This exists so
# that "it does not build for wasm32 on my machine" is a solved problem rather
# than a reason to leave the wasm-gated half of `election_config` unchecked
# locally — which is exactly what happened, twice.

LLVM_PREFIX="${LLVM_PREFIX:-$(brew --prefix llvm 2>/dev/null || echo /opt/homebrew/opt/llvm)}"

if [ ! -x "$LLVM_PREFIX/bin/clang" ]; then
    echo "No clang at $LLVM_PREFIX/bin/clang. Run: brew install llvm" >&2
    return 1 2>/dev/null || exit 1
fi

if ! "$LLVM_PREFIX/bin/clang" --print-targets | grep -q wasm32; then
    echo "$LLVM_PREFIX/bin/clang cannot target wasm32." >&2
    echo "Apple's clang cannot; Homebrew's and Nix's can." >&2
    return 1 2>/dev/null || exit 1
fi

# The resource directory is versioned by clang's major version, and Homebrew
# bumps it. Read it rather than pinning a number, which is what the flake does
# with `CLANG_MAJOR_VERSION` and what would otherwise break on the next upgrade.
CLANG_MAJOR="$(
    "$LLVM_PREFIX/bin/clang" --version |
        sed -n 's/.*clang version \([0-9]*\).*/\1/p' | head -1
)"
RESOURCE_DIR="$LLVM_PREFIX/lib/clang/$CLANG_MAJOR"

if [ ! -d "$RESOURCE_DIR" ]; then
    echo "No resource directory at $RESOURCE_DIR" >&2
    return 1 2>/dev/null || exit 1
fi

export CC_wasm32_unknown_unknown="$LLVM_PREFIX/bin/clang"
export AR_wasm32_unknown_unknown="$LLVM_PREFIX/bin/llvm-ar"
# `-isystem` for stddef.h and friends, `-resource-dir` so clang finds its own
# builtins for a target its driver was not configured around. The three
# optimisation flags are the flake's, kept so a local build produces comparable
# output rather than a differently-optimised one.
export CFLAGS_wasm32_unknown_unknown="-isystem $RESOURCE_DIR/include -resource-dir $RESOURCE_DIR -O3 -ffunction-sections -fdata-sections -fno-exceptions"

echo "wasm32 C toolchain: $CC_wasm32_unknown_unknown (clang $CLANG_MAJOR)"
