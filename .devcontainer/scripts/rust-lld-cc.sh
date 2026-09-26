#!/bin/sh
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Linker for development builds on aarch64 Linux, where Rust still links with
# GNU ld: the C compiler driver with the LLD that ships with the Rust toolchain,
# many times faster on the services' debug binaries. rustc runs its linker with
# the toolchain's lib/rustlib/<host>/bin first on PATH, which holds rust-lld and
# the gcc-ld directory with its ld.lld. Without them, the default linker runs.
if lld=$(command -v rust-lld) && [ -x "${lld%/*}/gcc-ld/ld.lld" ]; then
    exec cc -fuse-ld=lld -B"${lld%/*}/gcc-ld" "$@"
fi
exec cc "$@"
