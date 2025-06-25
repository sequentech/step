{ pkgs, ... }:

{
  # https://devenv.sh/basics/
  env.GREET = "devenv";

  # https://devenv.sh/packages/
  packages = with pkgs; [
    git
    hasura-cli
    reuse
    openssl
    postgresql_15
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

    cargo-watch

    python3
    python3Packages.virtualenvwrapper

    # for parsing docker-compose.yml
    yq
  ];

  # https://devenv.sh/scripts/
  scripts.hello.exec = "echo hello from $GREET";

  enterShell = ''
    set -a
    source .devcontainer/.env
    export LD_LIBRARY_PATH=${pkgs.openssl.out}/lib:$LD_LIBRARY_PATH
    export PATH=/workspaces/step/packages/step-cli/rust-local-target/release:$PATH
    set +a
  '';

  # https://devenv.sh/languages/
  languages.rust = {
    enable = true;
    # https://devenv.sh/reference/options/#languagesrustchannel
    channel = "nightly";
    toolchain.rust-src = pkgs.rustPlatform.rustLibSrc;
  };

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
