# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
cargo run --release --bin demo_tool -- post-ballots --port=5432 --password=postgres --board-count $1 --ciphertexts $2 --num-trustees $3 --threshold $4
echo $2 >> stats.txt