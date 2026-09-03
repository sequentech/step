{ pkgs, inputs, ... }:

# Check docs/docusaurus/docs/07-developers/11-updates/updating-rust-version.md on how to update rust version.
let
  rustOverlay = import (builtins.fetchTarball {
    url = "https://github.com/oxalica/rust-overlay/archive/107c334f141854f563f8adf1db781dc453d92639.tar.gz";
    sha256 = "138jwq564qji7dc5yav2j2c1c1mr65smqqk00mni9lvqhx0n45w4";
  });

  pkgsCrates = import inputs.nixpkgs-crates {
    inherit (pkgs.stdenv.hostPlatform) system;
  };

  pkgs' = pkgs.extend rustOverlay;

  rustStable = pkgs'.rust-bin.stable."1.96.0".default.override {
    targets    = [ "wasm32-unknown-unknown" "wasm32-wasip1" "wasm32-wasip2"];
    extensions = [ "rust-src" "rust-analyzer-preview" ];
  };

  # The wasm-bindgen CLI and the wasm-bindgen crate versions must match exactly.
  # Crates are fetched from the static CDN: crates.io's /api/v1/.../download
  # endpoint answers nix's downloader with HTTP 403.
  mkWasmBindgenCli = { version, sha256, cargoHash }: pkgsCrates.rustPlatform.buildRustPackage {
    pname = "wasm-bindgen-cli";
    inherit version cargoHash;
    src = builtins.fetchTarball {
      url = "https://static.crates.io/crates/wasm-bindgen-cli/wasm-bindgen-cli-${version}.crate";
      inherit sha256;
    };
    nativeBuildInputs = [ pkgsCrates.pkg-config ];
    buildInputs = [ pkgsCrates.openssl ]
      ++ pkgsCrates.lib.optionals pkgsCrates.stdenv.isDarwin [ pkgsCrates.curl ];
    doCheck = false;
  };

  # Pinned to the wasm-bindgen crate version both Cargo workspaces use
  # (packages/Cargo.toml and packages/wbraid/Cargo.toml: =0.2.123).
  wasm-bindgen-cli-pinned = mkWasmBindgenCli {
    version = "0.2.123";
    sha256 = "12xdns7cvnz0j26i9kryxggylsslkqs5l2b6lppfkv1bic8q0rya";
    cargoHash = "sha256-d7x6gtx5OqEE4MyT6yjYn/qtgjx7GroTpXJewnBV2dU=";
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

    # Binary Rust
    rustStable

    # AWS
    (aws-sam-cli.overridePythonAttrs { doCheck = false; })

    git
    hasura-cli
    reuse
    openssl
    glibc
    openssh
    postgresql_18
    python3
    openssh

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
    cargo-license
    cargo-audit

    wasm-pack
    wasm-bindgen-cli-pinned

    python3
    python3Packages.virtualenvwrapper

    # for parsing docker-compose.yml
    yq

    minio-client

    # AI. Note, requires allowUnfree: true in devenv.yaml
    claude-code

    # for plugins
    cargo-component
  ];

  # https://devenv.sh/scripts/
  scripts.hello.exec = "echo hello from $GREET";

    enterShell = ''
    set -a
    source .devcontainer/.env
    export LD_LIBRARY_PATH=${pkgs.openssl.out}/lib:$LD_LIBRARY_PATH
    export PATH=/workspaces/step/packages/step-cli/rust-local-target/release:$PATH
    set +a

    export RUST_SRC_PATH=${rustStable}/lib/rustlib/src/rust/library
  '';


  languages.java = {
    enable = true;
    maven = {
      enable = true;
    };
  };

  # https://devenv.sh/git-hooks/
  git-hooks.hooks = {
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