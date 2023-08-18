#!/bin/bash
set -ex
pwd
export CODESPACE_VSCODE_FOLDER=$(pwd)

# create .cargo/ dir if it doesn't exist
if [ ! -d "${CODESPACE_VSCODE_FOLDER}/.cargo/" ]
then
    mkdir "${CODESPACE_VSCODE_FOLDER}/.cargo/"
fi

# configure local dependencies
cat <<EOF > "${CODESPACE_VSCODE_FOLDER}/.cargo/config.toml"
[patch.'https://github.com/sequentech/strand']
strand = { path = "strand", features= ["rayon"] }
EOF

# download local dependencies
if [ ! -d strand ]
then
    git clone https://github.com/sequentech/strand
fi
cd strand
git checkout v0.2.0
cd ..

# finally, continue with the normal script
nix develop --command bash -c "nix build -vv -L && \
cargo build --all-features && \
cargo test --all-features --package=bulletin-board && \
echo Environment Built"
echo $?
