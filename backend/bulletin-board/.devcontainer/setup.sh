#!/bin/bash
set -ex
pwd
source ~/.bashrc
nix show-config
nix develop --command bash -c "nix build -vv -L && \
cargo build --all-features && \
cargo test --all-features --package=bulletin-board && \
echo Environment Built"
echo $?
