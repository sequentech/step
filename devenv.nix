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

    # immudb
    pkgs.go

    # To be able to use vim in the terminal
    pkgs.vim
  
    # utility for search
    pkgs.ack

    # docker utility
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

    # for development of immudb local store
    pkgs.sqlite
  ];

  # https://devenv.sh/scripts/
  scripts.hello.exec = "echo hello from $GREET";

  enterShell = ''
    hello
    git --version
    export COMPOSE_PROJECT_NAME=backend-services_devcontainer

    # Used by braid:
    export IMMUDB_USERNAME=immudb
    export IMMUDB_PASSWORD=immudb
    export IMMUDB_SERVER_URL=http://immudb:3322
    export IMMUDB_INDEX_DBNAME=boardsindex
    export IMMUDB_BOARD_DBNAME=bulletin_board
    export HASURA_GRAPHQL_ADMIN_SECRET=admin
    export HASURA_GRAPHQL_ENDPOINT=http://graphql-engine:8080
  '';

  # https://devenv.sh/languages/
  languages.rust = {
    enable = true;
    # https://devenv.sh/reference/options/#languagesrustversion
    version = "latest";
    packages.rust-src = pkgs.rustPlatform.rustLibSrc;
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
