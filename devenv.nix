{ pkgs, ... }:

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

    # to build the rug backend in strand/braid and wasm tooling
    wasm-pack
    gcc
    m4
    llvmPackages_19.clang-unwrapped
    llvmPackages_19.llvm
    llvmPackages_19.libclang

    # count line numbers
    scc

    # for development of immudb local store
    sqlite

    cargo-watch
    cargo-license
    cargo-audit

    python3
    python3Packages.virtualenvwrapper

    # for parsing docker-compose.yml
    yq

    # AI. Note, requires allowUnfree: true in devenv.yaml
    claude-code
  ];

  # https://devenv.sh/scripts/
  scripts.hello.exec = "echo hello from $GREET";

  enterShell = ''
    set -a
    source .devcontainer/.env
    export LD_LIBRARY_PATH=${pkgs.openssl.out}/lib:$LD_LIBRARY_PATH
    export PATH=/workspaces/step/packages/step-cli/rust-local-target/release:$PATH

    # Configure clang for wasm (similar to sequent-core devShell) so that
    # C code for wasm32-unknown-unknown (e.g. ring) can find standard headers
    export CC_wasm32_unknown_unknown=${pkgs.llvmPackages_19.clang-unwrapped}/bin/clang
    CLANG_MAJOR_VERSION=19
    CLANG_RESOURCE_DIR=${pkgs.llvmPackages_19.clang-unwrapped}/lib/clang/$CLANG_MAJOR_VERSION
    LIBCLANG_INCLUDE=${pkgs.llvmPackages_19.libclang.lib}/lib/clang/$CLANG_MAJOR_VERSION/include

    # Provide include paths and resource-dir for wasm32 C compilation.
    # Keep optimisation flags used in build scripts.
    export CFLAGS_wasm32_unknown_unknown="-isystem $LIBCLANG_INCLUDE -resource-dir $CLANG_RESOURCE_DIR -O3 -ffunction-sections -fdata-sections -fno-exceptions"
    export CPPFLAGS="-isystem $LIBCLANG_INCLUDE -resource-dir $CLANG_RESOURCE_DIR"

    set +a
  '';

  # https://devenv.sh/languages/
  languages.rust = {
    enable = true;
    # https://devenv.sh/reference/options/#languagesrustchannel
    channel = "stable";
    toolchain.rust-src = pkgs.rustPlatform.rustLibSrc;
  };

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
