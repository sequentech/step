{ pkgs, ... }:

{
  env = {
    # ereslibre: FIXME
    LOCAL_WORKSPACE_FOLDER = "/home/ereslibre/projects/sequentech/step";
    PATH = "/workspaces/step/packages/step-cli/rust-local-target/release:$PATH";
  };

  enterShell = ''
    source .env
  '';

  scripts = {
    # ereslibre: FIXME
    up.exec = ''
      devpod up --debug --recreate --ide none --ssh-config ~/.ssh-devpod/ssh.conf --devcontainer-path .devcontainer/devcontainer.json .
    '';

    build.exec = ''
      cd packages && yarn && yarn build:ui-core && yarn build:ui-essentials && yarn start:admin-portal
    '';

    down.exec = ''
      devpod delete
    '';

    cleanup.exec = ''
      docker ps -aq | xargs docker rm -f
    '';
  };

  # https://devenv.sh/packages/
  packages = with pkgs; [
    devpod
    git
    hasura-cli
    reuse
    openssl
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
    geckodriver

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

  # https://devenv.sh/languages/
  languages = {
    c.debugger = null;

    rust = {
      enable = true;
      # https://devenv.sh/reference/options/#languagesrustchannel
      channel = "nightly";
      # toolchain.rust-src = pkgs.rustPlatform.rustLibSrc;
    };

    java = {
      enable = true;
      maven = {
        enable = true;
      };
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

  # See full reference at https://devenv.sh/reference/options/
}
