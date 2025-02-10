#!/bin/bash -i
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -ex -o pipefail

if ! grep OPENWHISK_API_HOST .devcontainer/.env &> /dev/null; then
  cat <<EOF >> .devcontainer/.env
OPENWHISK_API_HOST="http://$(docker inspect openwhisk | jq -r '.[].Config.Hostname'):3233"
EOF
fi

source .devcontainer/.env

pushd packages/step-cli
cargo build --release
popd

source .devcontainer/scripts/config-cli.sh
