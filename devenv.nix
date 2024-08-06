{ pkgs, ... }:

{
  # https://devenv.sh/basics/
  env.GREET = "devenv";

  # https://devenv.sh/packages/
  packages = [
    pkgs.git
    pkgs.hasura-cli
    pkgs.reuse
    pkgs.openssl
    pkgs.postgresql_15
    pkgs.python3

    # immudb
    pkgs.go

    # To be able to use vim in the terminal
    pkgs.vim
  
    # utility for search
    pkgs.ack

    # docker utilities
    pkgs.dive

    # wget and curl
    pkgs.wget
    pkgs.curl

    # For frontend
    pkgs.yarn
    pkgs.nodejs_20
    pkgs.nodePackages.graphqurl

    # For protocol buffers
    pkgs.protobuf
    pkgs.iputils
    pkgs.chromium

    # to build the rug backend in strand/braid
    pkgs.gcc
    pkgs.m4

    # count line numbers
    pkgs.scc

    # for development of immudb local store
    pkgs.sqlite

    pkgs.cargo-watch
  ];

  # https://devenv.sh/scripts/
  scripts.hello.exec = "echo hello from $GREET";

  enterShell = ''
    set -a
    source .devcontainer/.env
    set +a
    curl -s https://get.extism.org/cli | sh --y 
    export PATH=/usr/local/bin:$PATH
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
