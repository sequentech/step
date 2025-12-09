# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
#/bin/bash
cargo run --bin m2 --release --features=monitor -- --host localhost --port=49153 --password=postgrespw
