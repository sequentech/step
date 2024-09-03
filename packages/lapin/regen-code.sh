#!/usr/bin/env bash

# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

main() {
    export LAPIN_CODEGEN_DIR="$(dirname "${0}" | xargs realpath)/src/"
    export LAPIN_CODEGEN_FILE="generated"

    cargo build --features=codegen-internal
    rustfmt "${LAPIN_CODEGEN_DIR}/${LAPIN_CODEGEN_FILE}.rs" --edition=2021
}

main "${@}"
