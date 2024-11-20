{ pkgs, ... }:

{
  # https://devenv.sh/packages/
  packages = with pkgs; [
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

  enterShell = ''
    set -a
    source .devcontainer/.env
    export LD_LIBRARY_PATH=${pkgs.openssl.out}/lib:$LD_LIBRARY_PATH
    export PATH=/workspaces/step/packages/step-cli/rust-local-target/release:$PATH
    set +a
    cat <<'EOF' | ${pkgs.bat}/bin/bat --language=markdown
      # Welcome to step!

      ## Start basic services

      - On the devcontainer, run `devenv up`.

      ## Forward ports

      - On your laptop, run `make HOST=host.example.com forward-ports`.

    EOF
  '';

  # https://devenv.sh/languages/
  languages = {
    java = {
      enable = true;
      maven.enable = true;
    };

    rust = {
      enable = true;
      # https://devenv.sh/reference/options/#languagesrustchannel
      channel = "nightly";
      toolchain.rust-src = pkgs.rustPlatform.rustLibSrc;
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

  processes = {
    admin-portal.exec = ''
      if [ ! -f .admin-portal-initialized ]; then
        pushd packages &> /dev/null
        yarn && yarn build:ui-core && yarn build:ui-essentials
        popd &> /dev/null
        touch .admin-portal-initialized
      fi
      pushd packages &> /dev/null
      yarn start:admin-portal
      popd &> /dev/null
    '';
  };

  scripts = {
    check-reuse.exec = "reuse lint";
    format-code.exec = "./.devcontainer/scripts/format-code.sh";
    init-cli.exec = ''
      cd /workspaces/step/packages/step-cli && \
        cargo build --release && \
        /workspaces/step/.devcontainer/scripts/config-cli.sh
    '';
    logs-b3.exec = ''
      cd .devcontainer && \
        COMPOSE_PROJECT_NAME=step_devcontainer \
        COMPOSE_PROFILES=base \
          docker compose logs -f --tail 200 b3
    '';
    logs-beat.exec = ''
      cd .devcontainer && \
        COMPOSE_PROJECT_NAME=step_devcontainer \
        COMPOSE_PROFILES=base \
          docker compose logs -f --tail 200 beat
    '';
    logs-harvest.exec = ''
      cd .devcontainer && \
        COMPOSE_PROJECT_NAME=step_devcontainer \
        COMPOSE_PROFILES=base \
          docker compose logs -f --tail 200 harvest
    '';
    logs-hasura.exec = ''
      cd .devcontainer && \
        COMPOSE_PROJECT_NAME=step_devcontainer \
        COMPOSE_PROFILES=base \
          docker compose logs -f --tail 200 graphql-engine
    '';
    logs-keycloak.exec = ''
      cd .devcontainer && \
        COMPOSE_PROJECT_NAME=step_devcontainer \
        COMPOSE_PROFILES=base \
          docker compose logs -f --tail 200 keycloak
    '';
    logs-minio.exec = ''
      cd .devcontainer && \
        COMPOSE_PROJECT_NAME=step_devcontainer \
        COMPOSE_PROFILES=base \
          docker compose logs -f --tail 200 minio
    '';
    logs-simplesaml.exec = ''
      cd .devcontainer && \
        COMPOSE_PROJECT_NAME=step_devcontainer \
        COMPOSE_PROFILES=base \
          docker compose logs -f --tail 200 simplesaml
    '';
    logs-trustees.exec = ''
      cd .devcontainer && \
        COMPOSE_PROJECT_NAME=step_devcontainer \
        COMPOSE_PROFILES=base \
          docker compose logs -f --tail 200 trustee1 trustee2
    '';
    logs-windmill.exec = ''
      cd .devcontainer && \
        COMPOSE_PROJECT_NAME=step_devcontainer \
        COMPOSE_PROFILES=base \
          docker compose logs -f --tail 200 windmill
    '';
    logs-restart-b3.exec = ''
      cd .devcontainer && \
        COMPOSE_PROJECT_NAME=step_devcontainer \
        COMPOSE_PROFILES=base \
          docker compose stop b3; \
            docker compose up -d --no-deps b3 && \
            docker compose logs -f --tail 100 b3
    '';
    logs-restart-beat.exec = ''
      cd .devcontainer && \
        COMPOSE_PROJECT_NAME=step_devcontainer \
        COMPOSE_PROFILES=base \
          docker compose stop beat; \
            docker compose up -d --no-deps beat && \
            docker compose logs -f --tail 100 beat
    '';
    logs-restart-harvest.exec = ''
      cd .devcontainer && \
        COMPOSE_PROJECT_NAME=step_devcontainer \
        COMPOSE_PROFILES=base \
          docker compose stop harvest; \
          docker compose up -d --no-deps harvest && \
            docker compose logs -f --tail 100 harvest
    '';
    logs-restart-hasura.exec = ''
      cd .devcontainer && \
        COMPOSE_PROJECT_NAME=step_devcontainer \
        COMPOSE_PROFILES=base \
          docker compose stop graphql-engine; \
          docker compose up -d --no-deps graphql-engine && \
            docker compose logs -f --tail 100 graphql-engine
    '';
    logs-restart-keycloak.exec = ''
      cd .devcontainer && \
        COMPOSE_PROJECT_NAME=step_devcontainer \
        COMPOSE_PROFILES=base \
          docker compose stop keycloak; \
          docker compose build keycloak && \
            docker compose up -d --no-deps keycloak && \
            docker compose logs -f --tail 100 keycloak
    '';
    logs-restart-minio.exec = ''
      cd .devcontainer && \
        COMPOSE_PROJECT_NAME=step_devcontainer \
        COMPOSE_PROFILES=base \
          docker compose stop configure-minio minio; \
          docker compose up --build -d --no-deps minio && \
          docker compose up --build -d --no-deps configure-minio && \
            docker compose logs -f --tail 100 configure-minio minio
    '';
    logs-restart-simplesaml.exec = ''
      cd .devcontainer && \
        COMPOSE_PROJECT_NAME=step_devcontainer \
        COMPOSE_PROFILES=base \
          docker compose stop simplesaml; \
          docker compose build simplesaml && \
            docker compose up -d --no-deps simplesaml && \
            docker compose logs -f --tail 100 simplesaml
    '';
    logs-restart-trustees.exec = ''
      cd .devcontainer && \
        COMPOSE_PROJECT_NAME=step_devcontainer \
        COMPOSE_PROFILES=base \
          docker compose stop trustee1 trustee2; \
            docker compose up -d --no-deps trustee1 trustee2 && \
            docker compose logs -f --tail 100 trustee1 trustee2
    '';
    logs-restart-windmill.exec = ''
      cd .devcontainer && \
        COMPOSE_PROJECT_NAME=step_devcontainer \
        COMPOSE_PROFILES=base \
          docker compose stop windmill; \
          docker compose up -d --no-deps windmill && \
            docker compose logs -f --tail 100 windmill
    '';
    report-locs.exec = "./scripts.loc.sh";
    start-voting-portal.exec = ''
      cd /workspaces/step/packages && yarn start:voting-portal
    '';
    start-ballot-verifier.exec = ''
      cd /workspaces/step/packages && yarn start:ballot-verifier
    '';
    update-env.exec = "./.devcontainer/scripts/initialize-command.sh";
    update-graphql.exec = "./.devcontainer/scripts/update-graphql.sh";
    update-vscode-settings.exec = "./.devcontainer/scripts/fix-vscode-settings-nix.sh";
  };

  # https://devenv.sh/integrations/dotenv/
  # Enable usage of the .env file for setting env variables
  # dotenv.enable = true;

  # https://devenv.sh/processes/
  # processes.ping.exec = "ping example.com";

  # See full reference at https://devenv.sh/reference/options/
}
