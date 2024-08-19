#!/bin/bash -i
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -ex -o pipefail

source .devcontainer/.env

cd packages/step-cli
cargo build --release

source .devcontainer/scripts/config-cli.sh