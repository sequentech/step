#!/bin/bash -i
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -ex -o pipefail

if git submodule update --init --recursive --depth 1; then
  echo "Submodules are successfully loaded and available in the workspace"
else
  echo "Failed to init submodules, they won't be available in the workspace"
fi


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
