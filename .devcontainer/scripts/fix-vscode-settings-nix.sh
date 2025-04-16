#!/bin/bash -i
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


    // Use the correct sysroot setup by devenv
    "rust-analyzer.cargo.sysroot": "$(rustc --print sysroot)",
    // echo /nix/store/*-openjdk-*/lib/openjdk
    "java.jdt.ls.java.home": "$java_home"
}
EOF

cat << EOF
########################################################
file '.vscode/settings.local.json' generated.
########################################################
EOF
