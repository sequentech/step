# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
# hard coded for trustees = 3
cargo run --release --bin demo_tool -- init-protocol --port=5432 --password=postgres --board-count $1
rm -f demo/1/message_store/*
rm -f demo/2/message_store/*
rm -f demo/3/message_store/*