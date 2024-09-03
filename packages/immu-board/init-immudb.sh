#!/bin/sh
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

cp -rf /workspaces/step/.devcontainer/keycloak/import /import
cp -rf /workspaces/step/.devcontainer/.env .env
cargo run --bin bb_helper -- --server-url ${IMMUDB_SERVER_URL} --username ${IMMUDB_USER} --password ${IMMUDB_PASSWORD} --index-dbname ${IMMUDB_INDEX_DB} --board-dbname ${IMMUDB_BOARD_DB_NAME} --cache-dir /tmp/immu-board upsert-init-db