#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
ROOT_DIR=$SCRIPT_DIR/..

pushd $ROOT_DIR/.devcontainer &> /dev/null
if [[ ! -f .env ]]; then
    cp .env.development .env
fi
docker compose up -d --build
popd &> /dev/null
pushd $ROOT_DIR &> /dev/null
devenv shell -- sh -c 'cd packages && yarn && yarn build:ui-core && yarn build:ui-essentials'
devenv shell -- sh -c 'cd packages && yarn start:admin-portal &'
devenv shell -- sh -c 'cd packages && yarn start:voting-portal &'
echo "Waiting for http://localhost:3000 to become ready..."
npx wait-on http://localhost:3000 --timeout 30m
echo "Waiting for http://localhost:3002 to become ready..."
npx wait-on http://localhost:3002 --timeout 30m
popd &> /dev/null