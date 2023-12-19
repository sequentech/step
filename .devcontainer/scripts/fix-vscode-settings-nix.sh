#!/bin/bash -i

set -ex -o pipefail

# Get the PATH for cargo
cargo_path=$(which cargo)
cargo_dir=$(dirname "$cargo_path")

# Get PKG_CONFIG_PATH
pkg_config_path=$(echo $PKG_CONFIG_PATH)

# Get RUST_SRC_PATH
rust_src_path=$(echo $RUST_SRC_PATH | sed 's|\(.*rustlib/src/\).*|\1|')

java_home=$(echo /nix/store/*-openjdk-*/lib/openjdk)

# Add below

# Generate settings.fix-nix.json
cat << EOF > '.vscode/settings.fix-nix.json'
{
    "rust-analyzer.server.extraEnv": {
        // See https://github.com/sequentech/backend-services/wiki/Running-tests-without-triggering-full-rebuilds
        // "CARGO_TARGET_DIR": "rust-analyzer-target",

        // which cargo
        "PATH": "$cargo_dir",

        // echo $PKG_CONFIG_PATH
        "PKG_CONFIG_PATH": "$pkg_config_path",

        // echo $RUST_SRC_PATH | sed 's|\(.*rustlib/src/\).*|\1|'
        "RUST_SRC_PATH": "$rust_src_path"
    },

    // echo /nix/store/*-openjdk-*/lib/openjdk
    "java.jdt.ls.java.home": "$java_home"
}
EOF

echo "settings.fix-nix.json generated in .vscode directory."
