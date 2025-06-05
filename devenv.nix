{ pkgs, ... }:
# ── 1. pin rust-overlay ──────────────────────────────────────────────
let
  rustOverlay = import (builtins.fetchTarball {
    url    = "https://github.com/oxalica/rust-overlay/archive/c52e346aedfa745564599558a096e88f9a5557f9.tar.gz";
    sha256 = "1m3925fwf7hq3vcdn9fl554mzip6y4rqbrq7jb377h2l5rq6p9nd";
  });

  #Extend the *existing* pkgs set so all other packages still come from it
  pkgs' = pkgs.extend rustOverlay;
  
  rustToolchain = pkgs'.rust-bin.stable.latest.default.override {
    targets    = [ "wasm32-unknown-unknown" ];
    extensions = [ "rust-src" ];
  };
in
{
  # https://devenv.sh/basics/
  env = {
    REGISTRY = "localhost:5000";
    OPENWHISK_BASIC_AUTH = "23bc46b1-71f6-4ed5-8c54-816aa4f8c502:123zO3xZCLrMN6v2BKK1dXYFpXlPkccOFqm12CdAsMgRU4VrNZ9lyGVCGuMDGIwP";
    # NOTE(ereslibre): You will find this Base Image duplicated in
    # multiple places; we know it's a pinned version that works to
    # render PDF with our current version of headless_chrome. The
    # places where this pinned version is duplicated is either because
    # they don't allow to use environment variables as an input, or
    # because they don't run within the devenv environment.
    ALPINE_LAMBDA_BASE_IMAGE = "alpine:3.17@sha256:8fc3dacfb6d69da8d44e42390de777e48577085db99aa4e4af35f483eb08b989";
  };

  # https://devenv.sh/packages/
  packages = with pkgs; [

    # Binary Rust tool-chain
    rustToolchain

    # AWS
    (aws-sam-cli.overridePythonAttrs { doCheck = false; })

    git
    hasura-cli
    reuse
    openssl
    glibc
    openssh
    postgresql_15
    python3

    # immudb
    go

    # To be able to use vim in the terminal
    vim

    # utility for search
    ack

    # docker utilities
    dive

    # wget and curl
    wget
    curl

    # For frontend
    yarn
    nodejs_20
    nodePackages.graphqurl

    # For protocol buffers
    protobuf
    iputils
    geckodriver
    firefox

    # to build the rug backend in strand/braid
    gcc
    m4

    # count line numbers
    scc

    # for development of immudb local store
    sqlite

    # rust dependencies
    cargo-watch
    wasm-pack
    wasm-bindgen-cli

    python3
    python3Packages.virtualenvwrapper

    # for parsing docker-compose.yml
    yq

    minio-client
  ];

  # https://devenv.sh/scripts/
  scripts.hello.exec = "echo hello from $GREET";

    enterShell = ''
    set -a
    source .devcontainer/.env
    export LD_LIBRARY_PATH=${pkgs.openssl.out}/lib:$LD_LIBRARY_PATH
    export PATH=/workspaces/step/packages/step-cli/rust-local-target/release:$PATH
    set +a
    export RUSTC=${rustToolchain}/bin/rustc
  '';


  languages.java = {
    enable = true;
    maven = {
      enable = true;
    };
  };

  # https://devenv.sh/pre-commit-hooks/
  pre-commit.hooks = {
    clippy.enable = false;
    rustfmt.enable = false;
    reuse = {
      enable = false;
      name = "Reuse license headers";
      entry = "${pkgs.reuse}/bin/reuse lint";
      pass_filenames = false;
    };
  };

  # https://devenv.sh/integrations/dotenv/
  # Enable usage of the .env file for setting env variables
  # dotenv.enable = true;

  # https://devenv.sh/processes/
  # processes.ping.exec = "ping example.com";

  # See full reference at https://devenv.sh/reference/options/
}
