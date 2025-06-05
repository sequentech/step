#!/usr/bin/env bash
# Rebuild <crate> into Wasm on every change.
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -eo pipefail

cargo watch -q -c \
  -w src \
  -x 'build --target wasm32-unknown-unknown'
