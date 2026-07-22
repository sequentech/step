# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
cargo run --release --bin demo_tool -- gen-configs --port=5432 --password=postgrespw --num-trustees=$1 --threshold=$2