#!/bin/bash
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -ex -o pipefail

# Get the PATH for cargo
cargo_path=$(which cargo)
cargo_dir=$(dirname "$cargo_path")
devenv_profile=$(echo /nix/store/*-devenv-profile/bin)

# Get PKG_CONFIG_PATH
pkg_config_path=$(echo $PKG_CONFIG_PATH)

# Get RUST_SRC_PATH
rust_src_path=$(echo $RUST_SRC_PATH | sed 's|/lib/rustlib/src/rust/library$||')

java_home=$(echo /nix/store/*-openjdk-*/lib/openjdk)

# Add below
# Generate `.vscode/settings.local.json`
cat << EOF > '.vscode/settings.local.json'
{
    "rust-analyzer.server.extraEnv": {
        // See https://github.com/sequentech/step/wiki/Running-tests-without-triggering-full-rebuilds
        "CARGO_TARGET_DIR": "rust-local-target",

        // which cargo
        "PATH": "/bin:$devenv_profile:$cargo_dir",

        // echo \$PKG_CONFIG_PATH
        "PKG_CONFIG_PATH": "$pkg_config_path",

        // echo \$RUST_SRC_PATH | sed 's|\(.*rustlib/src/\).*|\1|'
        "RUST_SRC_PATH": "$rust_src_path"
    },

    // echo /nix/store/*-openjdk-*/lib/openjdk
    "java.jdt.ls.java.home": "$java_home"
}
EOF

cat << EOF
########################################################
file '.vscode/settings.local.json' generated.
########################################################
EOF

source .devcontainer/.env

pushd packages/step-cli
cargo build --release
popd

if [ -z "$SUPER_ADMIN_TENANT_ID" ] || [ -z "$HASURA_ENDPOINT" ] || [ -z "$KEYCLOAK_URL" ] || [ -z "$KEYCLOAK_ADMIN" ] || [ -z "$KEYCLOAK_CLI_CLIENT_ID" ] || [ -z "$KEYCLOAK_CLI_CLIENT_SECRET" ]; then
    echo "missing default environments for auto config"
else
    seq step config --tenant-id "$SUPER_ADMIN_TENANT_ID" --endpoint-url "$HASURA_ENDPOINT" --keycloak-url "$KEYCLOAK_URL" --keycloak-user "$KEYCLOAK_ADMIN" --keycloak-password "$KEYCLOAK_ADMIN" --keycloak-client-id "$KEYCLOAK_CLI_CLIENT_ID" --keycloak-client-secret "$KEYCLOAK_CLI_CLIENT_SECRET"
fi