# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

psql postgres://postgres:admin@localhost:5432 -f .\src\scripts\database.sql
cargo run --bin setup -- --reset --old-indexes
# cargo run --bin setup -- --reset
cargo run --bin main --release