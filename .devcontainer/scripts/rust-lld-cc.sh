#!/bin/sh
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# rustc places its toolchain linker directory on PATH. Use that toolchain's
# gcc-compatible LLD driver when present, preserving the compiler's arguments
# and debug information. An unavailable LLD falls back to cc; a failed link
# keeps its failure status instead of retrying with a different linker.
if lld=$(command -v rust-lld) && [ -x "${lld%/*}/gcc-ld/ld.lld" ]; then
    exec cc -fuse-ld=lld -B"${lld%/*}/gcc-ld" "$@"
fi
exec cc "$@"
