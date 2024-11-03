#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

if [[ -z "$STEP_VERSION" || -z "$STEP_HASH" ]]; then
    echo 'Export $STEP_VERSION envvar with the tagged version to package, and $STEP_HASH'
    exit 1
fi

TODAY="$(date '+%Y-%m-%d')"
SCRIPT_PATH="$(cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P)"
PROJECT_ROOT=$(realpath "$SCRIPT_PATH/..")
AIRGAPPED_ARTIFACTS_ROOT="$PROJECT_ROOT/airgapped-artifacts"
AIRGAPPED_ARTIFACTS_TODAY="$AIRGAPPED_ARTIFACTS_ROOT/$TODAY"
IMAGE_ARTIFACTS_PATH="$AIRGAPPED_ARTIFACTS_TODAY/images"
DELIVERABLE_TARBALL="$AIRGAPPED_ARTIFACTS_ROOT/$TODAY.tar"

info() {
    echo "(info) $1"
}

fatal() {
    echo "(fatal) $1"
    exit 1
}

docker-compose-airgap-preparation() {
    tmpfile=$(mktemp)
    sed "s/STEP_VERSION/$STEP_VERSION/g" $PROJECT_ROOT/.devcontainer/docker-compose-airgap-preparation.yml > $tmpfile
    echo $tmpfile
}

all-images() {
    yq -r '.services[].image' < $(docker-compose-airgap-preparation) | grep -vwE 'null|devenv' | sort | uniq
}

filesystem-friendly-image-name() {
    echo "$1" | sed -r 's|[/:]|-|g'
}

archive-image-artifact() {
    # Base64 has characters such as '=' that are invalid in some
    # filesystems. Use Base32 instead; longer filenames but safer.
    local image_artifact_path="$IMAGE_ARTIFACTS_PATH/$(filesystem-friendly-image-name "$1").tar"
    info "Archiving image artifact $1 into $image_artifact_path"
    docker save "$1" > $image_artifact_path
}

build-images() {
    docker compose -f $PROJECT_ROOT/.devcontainer/docker-compose-airgap-preparation.yml --profile full build
}

pull-images() {
    docker compose -f $(docker-compose-airgap-preparation) --profile full pull --ignore-pull-failures
}

add-dotenv-to-tarball() {
    tmpdir=$(mktemp -d)
    cat <<'EOF' | sed "s/STEP_VERSION/$STEP_VERSION/g" | sed "s/STEP_HASH/$STEP_HASH/g" > $tmpdir/.env
# SPDX-FileCopyrightText: 2023-2024 Eduardo Robles <edu@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

################################################################################
# This is the default docker-compose profile used by the Dev Container
# https://docs.docker.com/compose/profiles/
COMPOSE_PROFILES=full

# And the project name, also for docker
COMPOSE_PROJECT_NAME=step_devcontainer

################################################################################
## Rust Build config
#
# Some Rust related build env vars below. Used only for the development
# environment. This is not relevant for production.

# We use `RUSTFLAGS=-Awarnings` in windmill etc to hide warnings during the
# builds and rebuilds that happen during development
RUSTFLAGS=-Awarnings
CARGO_TERM_COLOR=always

# This is so that the docker builds don't affect local development. In docker,
# we use the normal target/ dir. This var is loaded by devenv.nix not by docker
# services.
CARGO_TARGET_DIR=rust-local-target

# Get full backtraces. This var is loaded by devenv.nix not by docker services.
RUST_BACKTRACE=full

################################################################################
# App version and hash:
APP_VERSION="STEP_VERSION"
APP_HASH="STEP_HASH"

################################################################################
# General AWS deployment configuration. This is used for MINIO/S3 and for AWS
# SES, AWS SNS, etc
AWS_REGION=us-east-1

################################################################################
# AWS S3 related vars. minio is compatible with AWS S3 and we use it for
# the DevContainer development environment.
AWS_S3_ROOT_USER=minio_user
AWS_S3_ROOT_PASSWORD=minio_pass
AWS_S3_ACCESS_KEY=LZAw7hwBziRjwAhfP6Xi
AWS_S3_ACCESS_SECRET=4x8krlfXgEquxp9KhlCrCdkrECrszGQQlJa5nGct

# Don't set these two for production - we'll just use default AWS URIs instead
AWS_S3_PRIVATE_URI=http://minio:9000
AWS_S3_PUBLIC_URI=http://127.0.0.1:9000
AWS_S3_BUCKET=election-event-documents
AWS_S3_PUBLIC_BUCKET=public
AWS_S3_UPLOAD_EXPIRATION_SECS="120"
AWS_S3_FETCH_EXPIRATION_SECS="3600"
# Max upload file size. Defaulting to 10MiB = 10*1024*1024 = 10_485_760
AWS_S3_MAX_UPLOAD_BYTES="10485760"
# Cache policy for jwks files in S3.
AWS_S3_JWKS_CACHE_POLICY="max-age=30"
################################################################################
# harvest specific configuration
HARVEST_PORT="8400"

################################################################################
# This is the endpoint to connect to hasura
HASURA_ENDPOINT=http://graphql-engine:8080/v1/graphql

HASURA_PG_PASSWORD=postgrespassword
HASURA_PG_USER=postgres
HASURA_PG_PORT="5432"
HASURA_PG_DBNAME="postgres"
HASURA_PG_HOST="postgres"

# postgres database to store Hasura metadata
HASURA_GRAPHQL_METADATA_DATABASE_URL="postgres://${HASURA_PG_USER}:${HASURA_PG_PASSWORD}@${HASURA_PG_HOST}:${HASURA_PG_PORT}/${HASURA_PG_DBNAME}"

# this env var can be used to add the above postgres database to Hasura
# as a data source. this can be removed/updated based on your needs
HASURA_PG_DATABASE_URL="postgres://${HASURA_PG_USER}:${HASURA_PG_PASSWORD}@${HASURA_PG_HOST}:${HASURA_PG_PORT}/${HASURA_PG_DBNAME}"

## enable the console served by server
HASURA_GRAPHQL_ENABLE_CONSOLE="true"

## enable debugging mode. It is recommended to disable this in production
HASURA_GRAPHQL_DEV_MODE="true"

# https://hasura.io/docs/latest/deployment/graphql-engine-flags/config-examples/#console-assets-on-server
## to run console offline (i.e load console assets from server instead of CDN)
HASURA_GRAPHQL_CONSOLE_ASSETS_DIR="/srv/console-assets"

HASURA_GRAPHQL_ENABLED_LOG_TYPES="startup, http-log, webhook-log, websocket-log, query-log"

HASURA_GRAPHQL_METADATA_DEFAULTS='{"backend_configs":{"dataconnector":{"athena":{"uri":"http://data-connector-agent:8081/api/v1/athena"},"mariadb":{"uri":"http://data-connector-agent:8081/api/v1/mariadb"},"mysql8":{"uri":"http://data-connector-agent:8081/api/v1/mysql"},"oracle":{"uri":"http://data-connector-agent:8081/api/v1/oracle"},"snowflake":{"uri":"http://data-connector-agent:8081/api/v1/snowflake"}}}}'

# keycloak jwks endpoint
HASURA_GRAPHQL_JWT_SECRET='{"jwk_url": "http://minio:9000/public/certs.json"}'

# Used by Hasura action to point to harvest
HARVEST_DOMAIN="harvest:${HARVEST_PORT}"

################################################################################
# Information related to connection to immudb
IMMUDB_USER=immudb
IMMUDB_PASSWORD=immudb
IMMUDB_LOGS_DB=defaultdb
IMMUDB_HOST=immudb
IMMUDB_PORT=3322
IMMUDB_SERVER_URL="http://${IMMUDB_HOST}:${IMMUDB_PORT}"
# We need an initial index db
IMMUDB_INDEX_DB=indexdb
IMMUDB_BOARD_DB_NAME=33f18502a67c48538333a58630663559

################################################################################
# trustee/braid configuration
TRUSTEE1_CONFIG=/opt/braid/trustee1.toml
TRUSTEE2_CONFIG=/opt/braid/trustee2.toml
TRUSTEE3_CONFIG=/opt/braid/trustee3.toml
IGNORE_BOARDS=tenanttest,tenant90505c8a23a94cdfa26b4e19f6a097d5eventda4f7c9afd044da2a3afb032e80c1d7c

################################################################################
# Information related to the AWS Secrets Manager or Vault. We will migrate to
# support both Vault or AWS Secret manager soon.

# Defines the SECRETS Backend to use.
# Allowed values:
# - "HashiCorpVault" for Hashicorp Vault
# - "AwsSecretManager" for AWS Secret Manager
SECRETS_BACKEND=HashiCorpVault

# Only used when SECRETS_BACKEND=AwsSecretManager
# Prefix of all the keys used by the AWS Secret Manager.
# Rationale: Each AWS Region share the same AWS Secret Manager Key space, even
# among different deployments. To easily seggregate keys for different
# environments, we use a key prefix.
AWS_SM_KEY_PREFIX=development_environment_name_

# Only used when SECRETS_BACKEND=HashiCorpVault
# Points to the Hashicorp Vault server URL
VAULT_SERVER_URL=http://vault:8201

# Only used when SECRETS_BACKEND=HashiCorpVault
# Hashicorp Vault Token
VAULT_TOKEN=hvs.s0djsk0LLBI19K0DkW4Fajs7

################################################################################
# RabbitMQ configuration

# These two default user and password variables is used for configuring the
# default users in the rabbitmq service, so it's devenv-only
RABBITMQ_DEFAULT_USER=guest
RABBITMQ_DEFAULT_PASS=guest

# This is the AMQP variable to post or read message into or from rabbitmq
AMQP_ADDR=amqp://rabbitmq:5672

################################################################################
# Keycloak related vars
# Keycloak Base URL
KEYCLOAK_URL=http://keycloak:8090
KC_HOSTNAME="127.0.0.1"
KC_HOSTNAME_STRICT="false"
KC_HTTP_PORT="8090"
KC_DB_USERNAME=postgres
KC_DB_PASSWORD=postgrespassword
KC_DB_SCHEMA=public
KC_DB_URL_HOST=postgres-keycloak
KC_DB_URL_PORT=5432
KC_DB_URL_DATABASE=postgres
KC_OTP_RESEND_INTERVAL="60"
# Please note `KC_DB` is the DB vendor, i.e. the type of db. Postgres in this
# case:
KC_DB=postgres

KEYCLOAK_ADMIN=admin
KEYCLOAK_ADMIN_PASSWORD=admin

# Client Id and secret used for the authentication flow
KEYCLOAK_CLIENT_ID=service-account
KEYCLOAK_CLIENT_SECRET=zh0GWEjbynXJDcpF1YipFXUleEKTQiO0

# Client Id and secret used for administrative tasks like creating a new realm
# or similar
KEYCLOAK_ADMIN_CLIENT_ID=admin
KEYCLOAK_ADMIN_CLIENT_SECRET=admin

# Path to the default configuration file when creating a new realm related to an
# election event. For production, this can be coming from a mounted volume, so
# that it can be changed without requiring a new OCI/Docker image.
KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH=/opt/keycloak/data/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-33f18502-a67c-4853-8333-a58630663559.json
# Path to the default configuration file when creating a new realm related to
# a tenant. For production, this can be coming from a mounted volume, so that
# it can be changed without requiring a new OCI/Docker image.
KEYCLOAK_TENANT_REALM_CONFIG_PATH=/opt/keycloak/data/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json

# This is the super-admin keycloak realm (and tenant id). When you login into
# the admin portal, you actually login to this Keycloak realm.
SUPER_ADMIN_TENANT_ID=90505c8a-23a9-4cdf-a26b-4e19f6a097d5
MAX_DIFF_LINES=500

# This is the name of the keycloak group voters need to be included in
KEYCLOAK_VOTER_GROUP_NAME=voter

################################################################################
# Configuration related to rust-based services. Basically, since we use Rocket
# ( see https://rocket.rs/ ), we can use these variable to configure the default
# address and port used to serve for these services (windmill, harvest, etc).
ROCKET_ADDRESS=0.0.0.0

# Log level for rust based services
LOG_LEVEL=info

# Windmill (and in the future also Harvest and Beat) need to directly connect to
# the databases. Right now it's only Keycloak DB, but in the future also to
# Hasura database directly. The configuration below is related to these
# connection settings.
#
# We're configuring the KEYCLOAK_DB object that has the type
# `deadpool_postgres::Config` using the `config` crate that allows us to parse
# configuration from environment variables. More information in the links below:
# - deadpool_postgres::Config:
#   https://docs.rs/deadpool-postgres/0.11.0/deadpool_postgres/struct.Config.html
# - config crate: https://crates.io/crates/config
#
# NOTE: that we cannot directly use a postgres:// connection string because it's
# not yet implemented by deadpool-postgres, for more info see the ticket below:
# https://github.com/bikeshedder/deadpool/issues/261
KEYCLOAK_DB__USER=${KC_DB_USERNAME}
KEYCLOAK_DB__PASSWORD=${KC_DB_PASSWORD}
KEYCLOAK_DB__HOST=${KC_DB_URL_HOST}
KEYCLOAK_DB__PORT=${KC_DB_URL_PORT}
KEYCLOAK_DB__DBNAME=${KC_DB_URL_DATABASE}
KEYCLOAK_DB__MANAGER__RECYCLING_METHOD=Verified

# Now we configure Hasura Db in the same manner as Keycloak Db:
HASURA_DB__USER=${HASURA_PG_USER}
HASURA_DB__PASSWORD=${HASURA_PG_PASSWORD}
HASURA_DB__HOST=${HASURA_PG_HOST}
HASURA_DB__PORT=${HASURA_PG_PORT}
HASURA_DB__DBNAME=${HASURA_PG_DBNAME}
HASURA_DB__MANAGER__RECYCLING_METHOD=Verified


LOW_SQL_LIMIT="1000"
DEFAULT_SQL_LIMIT="20"
DEFAULT_SQL_BATCH_SIZE="1000"

################################################################################
# This is the base url of the voting portal. This is used by windmill when
# generating urls for voters to vote during the sending of messages to voters
VOTING_PORTAL_URL=http://localhost:3000

################################################################################
# Configuration related to communications sent to voters, used by windmill.

# Variable used to configure the transport to use for sending emails.
# Allowed values:
# - "Console" which prints the email in the console log
# - "AwsSes" which sends the email using AWS SES
EMAIL_TRANSPORT_NAME=Console

# FROM address used when sending emails
EMAIL_FROM=info@sequentech.io

# Variable used to configure the transport to use for sending SMS messages.
# Allowed values:
# - "Console" which prints the email in the console log
# - "AwsSns" which sends the email using AWS SNS
SMS_TRANSPORT_NAME=Console

# JSON Configuration for AWS SNS. For example you can configure the SenderID. We
# Only support String kind of attributes.
# More information here: https://docs.aws.amazon.com/sns/latest/dg/sms_publish-to-phone.html#sms_publish_sdk
AWS_SNS_ATTRIBUTES='{"SenderID": "SEQUENT", "SMSType": "TRANSACTIONAL"}'

# Public Assets that gets uploaded on minio / s3 bucket
# Usecase: print ballot receipt to PDF, etc
PUBLIC_ASSETS_PATH="public-assets"
PUBLIC_ASSETS_LOGO_IMG="sequent-logo.svg"
PUBLIC_ASSETS_QRCODE_LIB="qrcode.min.js"
PUBLIC_ASSETS_VOTE_RECEIPT_TEMPLATE="vote_receipt.hbs"
PUBLIC_ASSETS_VOTE_RECEIPT_TEMPLATE_CONTENT="vote_receipt_content.hbs"
PUBLIC_ASSETS_VELVET_VOTE_RECEIPTS_TEMPLATE="velvet_vote_receipts.hbs"
PUBLIC_ASSETS_EML_BASE_TEMPLATE="eml_base.hbs"
VOTE_RECEIPT_TEMPLATE_TITLE="Ballot receipt - Sequentech"
VELVET_VOTE_RECEIPTS_TEMPLATE_TITLE="Vote receipts - Sequentech"

# uuids are replaced when you create or import an Election Event. This parameter
# allows you to avoid replacing certain uuids.
ELECTION_EVENT_FIXED_UUIDS=""

################################################################################
# Probe connection and path settings

IMMUDB_LOG_AUDIT_PROBE_ADDR=0.0.0.0:3030
IMMUDB_LOG_AUDIT_PROBE_LIVE_PATH=live

HARVEST_PROBE_ADDR=0.0.0.0:3030
HARVEST_PROBE_LIVE_PATH=live
HARVEST_PROBE_READY_PATH=ready

WINDMILL_PROBE_ADDR=0.0.0.0:3030
WINDMILL_PROBE_LIVE_PATH=live
WINDMILL_PROBE_READY_PATH=ready

BEAT_PROBE_ADDR=0.0.0.0:3030
BEAT_PROBE_LIVE_PATH=live
BEAT_PROBE_READY_PATH=ready

# Demo key
DEMO_PUBLIC_KEY="eh8l6lsmKSnzhMewrdLXEKGe9KVxxo//QsCT2wwAkBo"

#Voting portal countdwon policy default values
SECONDS_TO_SHOW_COUNTDOWN=60
SECONDS_TO_SHOW_ALERT=300


# CLI Usage
KEYCLOAK_CLI_CLIENT_ID=admin-portal
KEYCLOAK_CLI_CLIENT_SECRET=wBy8rpuKQxPWikQ3rIFv9g42t0WK0Xiu

#CloudFlare + Custom urls
CLOUDFLARE_ZONE=
CLOUDFLARE_API_KEY=
CUSTOM_URLS_IP_DNS_CONTENT=

# B3 configuration
B3_PG_HOST=postgres
B3_PG_PORT=5432
B3_PG_USER=postgres
B3_PG_PASSWORD=postgrespassword
B3_PG_DATABASE=b3
B3_BIND=0.0.0.0:50051
B3_URL=http://b3:50051
EOF

    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL .env
}


add-janitor-to-tarball() {
    JANITOR_PARENT="${PROJECT_ROOT}/packages/windmill/external-bin/"
    tar --append -C $JANITOR_PARENT --file=$DELIVERABLE_TARBALL janitor
}

add-database-init-to-tarball() {
    tmpdir=$(mktemp -d)
    mkdir -p $tmpdir/initdb
    cat <<'EOF' > $tmpdir/initdb/b3.sql
SELECT 'CREATE DATABASE b3'
    WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'b3')\gexec

\c b3

CREATE TABLE IF NOT EXISTS INDEX (
            id SERIAL PRIMARY KEY,
            board_name VARCHAR UNIQUE,
            is_archived BOOLEAN,
            cfg_id VARCHAR,
            threshold_no INT,
            trustees_no INT,
            last_message_kind VARCHAR,
            last_updated TIMESTAMP,
            message_count INT,
            batch_count INT DEFAULT 0,
            UNIQUE(board_name)
        );
EOF

    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL initdb
}

add-readme-to-tarball() {
    tmpdir=$(mktemp -d)
    cat <<'EOF' > $tmpdir/README.md
# Welcome to Sequent air-gapped environment

## Instructions

First decompress the folder

### Windows (x86-64)

#### 1. Enable WSL Feature

On the machine Open `Windows PowerShell` as Administrator
(`Run as Administrator`) and execute the following to  enable WSL 2:

```bash
dism /online /enable-feature /featurename:Microsoft-Windows-Subsystem-Linux /all /norestart
dism /online /enable-feature /featurename:VirtualMachinePlatform /all /norestart
```

#### 2. Install Desktop Desktop

Install Docker Desktop clicking the file `docker_desktop_installer.exe` inside the
`docker-desktop` folder.

Then restart the computer.

#### 3. Install the WSL Kernel Update

Install Docker Desktop clicking the file `wsl_update_x64.msi` inside the
`docker-desktop` folder.

#### 4. Install the Ubuntu WSL Distro

Now, you need to install the Ubuntu Linux distribution we will be using to run
the system.

Navigate to the folder `docker-desktop` and run the following command to install
the distribution:

```bash
# install ubuntu
Add-AppxPackage ubuntu.appx
```

### 5. Configure WSL defaults

Now let's configure WSL 2 as the default:

```bash
wsl --set-version Ubuntu 2
```

Then you need to set ubuntu as the default WSL linux distro:

```bash
# set ubuntu as the default distro
wsl --setdefault ubuntu
```

And check that this distro is actually the default by executing the following
command:


```bash
# list distros and ensure the default is ubuntu
wsl -l
```

The output should be something like:

```
Windows Subsystem for Linux Distributions:
Ubuntu (default)
docker-desktop
```

#### 6. Configure WSL in Docker Desktop

In Docker Desktop, activate the WSL integration in Docker Desktop Settings
(more info in https://docs.docker.com/desktop/wsl/):
- Enter into `Docker Desktop`
- Navigate to `Settings` (the gear button on the top blue header)
- Click `General` in the sidebar and ensure `Use WSL 2 based engine` is enabled
- Click in `Resources` in the sidebar and then click `WSL integration`
- Ensure `Enable integration with my default WSL distro` is enabled
- Ensure `ubuntu` is enabled under the
 `Enable integration with additional distros` section
- Click `Apply & restart` button if you had to apply any changes

#### 7. Executing Sequent Step Platform

In order to execute Sequent Step, you have to run the following command:

```bash
$ wsl sudo bash ./up
```

Then you need to go to

Once that it has been imported and started, you can visit the different services
at their endpoints:

- Admin portal: http://localhost:3002

### Linux/Mac (x86-64)

#### 1. Install Docker Desktop

- Docker Desktop installed

#### 2. Executing Sequent Step Platform

In order to execute Sequent Step Platform, you have to run the following
command as a root user:

```bash
sudo su -
$ ./up
```

Once that it has been imported and started, you can visit the different services
at their endpoints:

- Admin portal: http://localhost:3002
EOF
    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL README.md

}

add-docker-compose-to-tarball() {
    tmpdir=$(mktemp -d)
    cat <<'EOF' | sed "s/STEP_VERSION/$STEP_VERSION/g" | sed "s/STEP_HASH/$STEP_HASH/g" > $tmpdir/docker-compose.yml
# SPDX-FileCopyrightText: 2023-2024 Eduardo Robles <edu@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

services:
  simplesaml:
    profiles: ["full"]
    image: kenchan0130/simplesamlphp
    pull_policy: never
    container_name: simplesamlphp
    ports:
      - "8083:8080"
    volumes:
      - ./simplesaml/authsources.php:/var/www/simplesamlphp/config/authsources.php
      - ./simplesaml/saml20-sp-remote.php:/var/www/simplesamlphp/metadata/saml20-sp-remote.php

  # Needed to set up proper permissions for non root user in postgres
  postgres-volume-init:
    profiles: ["full", "base"]
    image: alpine:latest
    pull_policy: never
    container_name: postgres-volume-init
    volumes:
      - db_logs:/logs
      - keycloak_db_logs:/keycloak_logs
    user: root
    group_add:
      - '999'
    command: chown -R 999:999 /logs /keycloak_logs

  postgres:
    profiles: ["full", "base"]
    image: ghcr.io/sequentech/step/postgres:latest
    pull_policy: never
    container_name: postgres
    restart: unless-stopped
    volumes:
      - db_data:/var/lib/postgresql/data
      - db_logs:/logs
      - ./initdb:/docker-entrypoint-initdb.d
    environment:
      POSTGRES_PASSWORD: ${HASURA_PG_PASSWORD}
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
      timeout: 10s
      retries: 15
      start_period: 5s
    command: postgres -c 'config_file=/etc/postgresql/postgresql.conf'
    depends_on:
      - postgres-volume-init

  minio:
    profiles: ["full", "base"]
    container_name: minio
    image: minio/minio
    pull_policy: never
    ports:
      - "9000:9000"
      - "9001:9001"
    volumes:
      - minio_storage:/data
    environment:
      - MINIO_ROOT_USER=${AWS_S3_ROOT_USER}
      - MINIO_ROOT_PASSWORD=${AWS_S3_ROOT_PASSWORD}
      - MINIO_ACCESS_KEY=${AWS_S3_ACCESS_KEY}
      - MINIO_ACCESS_SECRET=${AWS_S3_ACCESS_SECRET}
    command: server --console-address ":9001" /data

  # used to add headers to minio
  minio-proxy:
    profiles: ["full", "base"]
    container_name: minio-proxy
    image: nginx:latest
    pull_policy: never
    ports:
      - "9002:9002"
    volumes:
      - ./minio/nginx:/etc/nginx/conf.d
    depends_on:
      - minio

  configure-minio:
    profiles: ["full", "base"]
    container_name: configure-minio
    image: sequentech.local/configure-minio
    pull_policy: never
    depends_on:
      - minio
    environment:
      - MINIO_ROOT_USER=${AWS_S3_ROOT_USER}
      - MINIO_ROOT_PASSWORD=${AWS_S3_ROOT_PASSWORD}
      - MINIO_ACCESS_KEY=${AWS_S3_ACCESS_KEY}
      - MINIO_ACCESS_SECRET=${AWS_S3_ACCESS_SECRET}
      - MINIO_PRIVATE_URI=${AWS_S3_PRIVATE_URI}
      - MINIO_PUBLIC_BUCKET=${AWS_S3_PUBLIC_BUCKET}
      - MINIO_BUCKET=${AWS_S3_BUCKET}
    entrypoint: /scripts/entrypoint.sh

  # hashicorp vault to store secrets
  vault:
    profiles: ["full", "base"]
    container_name: vault
    image: sequentech.local/vault
    pull_policy: never
    restart: on-failure:10
    #recommend way for docker-outside-of-docker is using devcontainer.json forwardPorts
    #More info: https://github.com/microsoft/vscode-dev-containers/blob/main/containers/docker-from-docker-compose/.devcontainer/docker-compose.yml#L28
    ports:
      - "8201:8201"
      - "8200:8200"
    environment:
      VAULT_API_ADDR: 'http://0.0.0.0:8200'
      VAULT_ADDR: 'http://0.0.0.0:8201'
      VAULT_UNSEAL_KEY: ciWE5G/CT7/uo5mfaGeRvSyuGRnbtijzvLDg3ru/jv0=
      VAULT_TOKEN: hvs.s0djsk0LLBI19K0DkW4Fajs7
    cap_add:
      - IPC_LOCK
    volumes:
      - vault-volume:/vault
    healthcheck:
      retries: 5
    entrypoint: /opt/vault/entrypoint.sh

  graphql-engine:
    profiles: ["full", "base"]
    image: hasura/graphql-engine:v2.33.1.cli-migrations-v3
    pull_policy: never
    container_name: hasura
    ports:
      - "8080:8080"
    restart: always
    volumes:
      - ./hasura/metadata:/hasura/metadata
      - ./hasura/migrations:/hasura/migrations
    environment:
      # applies migrations on start
      HASURA_GRAPHQL_MIGRATIONS_SERVER_TIMEOUT: 60
      HASURA_GRAPHQL_MIGRATIONS_DIR: /hasura/migrations
      HASURA_GRAPHQL_METADATA_DIR: /hasura/metadata
      ## postgres database to store Hasura metadata
      HASURA_GRAPHQL_METADATA_DATABASE_URL: ${HASURA_GRAPHQL_METADATA_DATABASE_URL}
      ## this env var can be used to add the above postgres database to Hasura as a data source. this can be removed/updated based on your needs
      PG_DATABASE_URL: ${HASURA_PG_DATABASE_URL}
      ## enable the console served by server
      # set to "false" to disable console
      HASURA_GRAPHQL_ENABLE_CONSOLE: ${HASURA_GRAPHQL_ENABLE_CONSOLE}
      ## enable debugging mode. It is recommended to disable this in production
      HASURA_GRAPHQL_DEV_MODE: ${HASURA_GRAPHQL_DEV_MODE}
      # https://hasura.io/docs/latest/deployment/graphql-engine-flags/config-examples/#console-assets-on-server
      ## uncomment next line to run console offline (i.e load console assets from server instead of CDN)
      HASURA_GRAPHQL_CONSOLE_ASSETS_DIR: ${HASURA_GRAPHQL_CONSOLE_ASSETS_DIR}
      HASURA_GRAPHQL_ENABLED_LOG_TYPES: ${HASURA_GRAPHQL_ENABLED_LOG_TYPES}
      HASURA_GRAPHQL_METADATA_DEFAULTS: ${HASURA_GRAPHQL_METADATA_DEFAULTS}
      # Hasura role for unauthorized users
      HASURA_GRAPHQL_UNAUTHORIZED_ROLE: "unauthorized"
      # keycloak jwks endpoint
      HASURA_GRAPHQL_JWT_SECRET: ${HASURA_GRAPHQL_JWT_SECRET}
      HASURA_GRAPHQL_ADMIN_SECRET: ${KEYCLOAK_ADMIN_CLIENT_SECRET}
      ACTIONS_ADMIN_SECRET: ${KEYCLOAK_ADMIN_CLIENT_SECRET}
      HARVEST_DOMAIN: ${HARVEST_DOMAIN}
    depends_on:
      data-connector-agent:
        condition: service_healthy
      postgres:
        condition: service_healthy

  data-connector-agent:
    profiles: ["full", "base"]
    container_name: data-connector-agent
    image: hasura/graphql-data-connector:v2.31.0
    pull_policy: never
    restart: always
    ports:
      - 8081:8081
    environment:
      QUARKUS_LOG_LEVEL: ERROR # FATAL, ERROR, WARN, INFO, DEBUG, TRACE
      ## https://quarkus.io/guides/opentelemetry#configuration-reference
      QUARKUS_OPENTELEMETRY_ENABLED: "false"
      ## QUARKUS_OPENTELEMETRY_TRACER_EXPORTER_OTLP_ENDPOINT: http://jaeger:4317
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8081/api/v1/athena/health"]
      interval: 5s
      timeout: 10s
      retries: 25
      start_period: 5s

  postgres-keycloak:
    profiles: ["full", "base"]
    image: ghcr.io/sequentech/step/postgres:latest
    pull_policy: never
    container_name: postgres-keycloak
    restart: unless-stopped
    volumes:
      - keycloak_db_data:/var/lib/postgresql/data
      - keycloak_db_logs:/logs
    command: postgres -c 'config_file=/etc/postgresql/postgresql.conf'
    environment:
      POSTGRES_PASSWORD: ${KC_DB_PASSWORD}
    healthcheck:
      test: ["CMD-SHELL", "pg_isready  -U postgres"]
      interval: 5s
      timeout: 10s
      retries: 15
      start_period: 5s

  keycloak:
    profiles: ["full", "base"]
    container_name: keycloak
    image: 581718213778.dkr.ecr.us-east-1.amazonaws.com/keycloak:STEP_VERSION
    pull_policy: never
    restart: always
    ports:
      - 8090:8090
    environment:
      KC_HOSTNAME: ${KC_HOSTNAME}
      KC_HOSTNAME_STRICT: ${KC_HOSTNAME_STRICT}
      KC_HTTP_PORT: ${KC_HTTP_PORT}
      KC_DB: ${KC_DB}
      KC_DB_USERNAME: ${KC_DB_USERNAME}
      KC_DB_PASSWORD: ${KC_DB_PASSWORD}
      KC_DB_SCHEMA: ${KC_DB_SCHEMA}
      KC_DB_URL_HOST: ${KC_DB_URL_HOST}
      KC_DB_URL_PORT: ${KC_DB_URL_PORT}
      KC_DB_URL_DATABASE: ${KC_DB_URL_DATABASE}
      KEYCLOAK_ADMIN: ${KEYCLOAK_ADMIN}
      KEYCLOAK_ADMIN_PASSWORD: ${KEYCLOAK_ADMIN_PASSWORD}
      KEYCLOAK_URL: ${KEYCLOAK_URL}
      KEYCLOAK_CLIENT_ID: ${KEYCLOAK_CLIENT_ID}
      SUPER_ADMIN_TENANT_ID: ${SUPER_ADMIN_TENANT_ID}
      KEYCLOAK_CLIENT_SECRET: ${KEYCLOAK_CLIENT_SECRET}
      KC_OTP_RESEND_INTERVAL: ${KC_OTP_RESEND_INTERVAL}
      HARVEST_DOMAIN: ${HARVEST_DOMAIN}
      APP_VERSION: ${APP_VERSION}
      APP_HASH: ${APP_HASH}
    volumes:
      # https://www.keycloak.org/server/containers#_importing_a_realm_on_startup
      - ./keycloak/import:/opt/keycloak/data/import:z
    # Below we're using the dummy email email sender provider but that's just for
    # development, in production we should still use the default SMTP email provider
    #
    # We are doing the same to configure the dummy sms sender provider
    #
    # More info here: https://www.keycloak.org/server/configuration-provider
    entrypoint: >
      /opt/keycloak/bin/kc.sh start-dev
      --features=preview
      --health-enabled=true
      --spi-user-profile-declarative-user-profile-read-only-attributes=area-id,tenant-id
      --spi-user-profile-declarative-user-profile-admin-read-only-attributes=sequent.admin-read-only.*
      -Dkeycloak.profile.feature.upload_scripts=enabled
      --spi-email-sender-provider=dummy
      --spi-email-sender-dummy-enabled=true
      --spi-email-sender-default-enabled=false
      --spi-sms-sender-provider=dummy
      --spi-sms-sender-dummy-enabled=true
      --spi-sms-sender-aws-enabled=false
      --import-realm
    depends_on:
      postgres-keycloak:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-f", "http://127.0.0.1:8090/health/live"]
      interval: 5s
      timeout: 10s
      retries: 25
      start_period: 5s

  admin-portal:
    profiles: ["full"]
    image: 581718213778.dkr.ecr.us-east-1.amazonaws.com/admin-portal:STEP_VERSION
    pull_policy: never
    container_name: admin-portal
    restart: always
    ports:
      - 3002:8000
    stdin_open: true
    environment:
        MAX_DIFF_LINES: ${MAX_DIFF_LINES}
        SECONDS_TO_SHOW_COUNTDOWN: ${SECONDS_TO_SHOW_COUNTDOWN}
        SECONDS_TO_SHOW_ALERT: ${SECONDS_TO_SHOW_ALERT}
        APP_VERSION: ${APP_VERSION}
        APP_HASH: ${APP_HASH}

  voting-portal:
    profiles: ["full"]
    image: 581718213778.dkr.ecr.us-east-1.amazonaws.com/voting-portal:STEP_VERSION
    pull_policy: never
    container_name: voting-portal
    depends_on:
      - admin-portal
    restart: always
    ports:
      - 3000:8000
    stdin_open: true
    environment:
        APP_VERSION: ${APP_VERSION}
        APP_HASH: ${APP_HASH}

  immudb:
    profiles: ["full", "base"]
    container_name: immudb
    image: 581718213778.dkr.ecr.us-east-1.amazonaws.com/immudb:STEP_VERSION
    pull_policy: never
    restart: always
    environment:
      - IMMUDB_PGSQL_SERVER=true
    ports:
      - 3322:3322 # immudb service
      - 3324:9497 # prometheus
      - 3325:8080 # web console
    volumes:
      - immudb_data:/var/lib/immudb
      - immudb_logs:/var/log/immudb
    depends_on:
      - immudb-init
    healthcheck:
      test: /usr/local/bin/immuadmin status
      interval: 30s
      timeout: 30s
      retries: 3

  harvest:
    profiles: ["full", "base"]
    stdin_open: true
    image: 581718213778.dkr.ecr.us-east-1.amazonaws.com/harvest:STEP_VERSION
    pull_policy: never
    volumes:
      - ./keycloak/import:/opt/keycloak/data/import
    environment:
      SUPER_ADMIN_TENANT_ID: ${SUPER_ADMIN_TENANT_ID}
      LOG_LEVEL: ${LOG_LEVEL}
      ROCKET_ADDRESS: ${ROCKET_ADDRESS}
      ROCKET_PORT: ${HARVEST_PORT}
      RUSTFLAGS: ${RUSTFLAGS}
      CARGO_TERM_COLOR: ${CARGO_TERM_COLOR}
      IMMUDB_USER: ${IMMUDB_USER}
      IMMUDB_PASSWORD: ${IMMUDB_PASSWORD}
      IMMUDB_SERVER_URL: ${IMMUDB_SERVER_URL}
      KEYCLOAK_DB__USER: ${KEYCLOAK_DB__USER}
      KEYCLOAK_DB__PASSWORD: ${KEYCLOAK_DB__PASSWORD}
      KEYCLOAK_DB__HOST: ${KEYCLOAK_DB__HOST}
      KEYCLOAK_DB__PORT: ${KEYCLOAK_DB__PORT}
      KEYCLOAK_DB__DBNAME: ${KEYCLOAK_DB__DBNAME}
      KEYCLOAK_DB__MANAGER__RECYCLING_METHOD: ${KEYCLOAK_DB__MANAGER__RECYCLING_METHOD}
      AMQP_ADDR: ${AMQP_ADDR}
      KEYCLOAK_URL: ${KEYCLOAK_URL}
      KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH: ${KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH}
      KEYCLOAK_TENANT_REALM_CONFIG_PATH: ${KEYCLOAK_TENANT_REALM_CONFIG_PATH}
      KEYCLOAK_ADMIN_CLIENT_ID: ${KEYCLOAK_ADMIN_CLIENT_ID}
      KEYCLOAK_ADMIN_CLIENT_SECRET: ${KEYCLOAK_ADMIN_CLIENT_SECRET}
      KEYCLOAK_CLIENT_ID: ${KEYCLOAK_CLIENT_ID}
      KEYCLOAK_CLIENT_SECRET: ${KEYCLOAK_CLIENT_SECRET}
      HASURA_ENDPOINT: ${HASURA_ENDPOINT}
      HASURA_DB__USER: ${HASURA_DB__USER}
      HASURA_DB__PASSWORD: ${HASURA_DB__PASSWORD}
      HASURA_DB__HOST: ${HASURA_DB__HOST}
      HASURA_DB__PORT: ${HASURA_DB__PORT}
      HASURA_DB__DBNAME: ${HASURA_DB__DBNAME}
      HASURA_DB__MANAGER__RECYCLING_METHOD: ${HASURA_DB__MANAGER__RECYCLING_METHOD}
      LOW_SQL_LIMIT: ${LOW_SQL_LIMIT}
      DEFAULT_SQL_LIMIT: ${DEFAULT_SQL_LIMIT}
      DEFAULT_SQL_BATCH_SIZE: ${DEFAULT_SQL_BATCH_SIZE}
      HARVEST_PROBE_ADDR: ${HARVEST_PROBE_ADDR}
      HARVEST_PROBE_LIVE_PATH: ${HARVEST_PROBE_LIVE_PATH}
      HARVEST_PROBE_READY_PATH: ${HARVEST_PROBE_READY_PATH}
      KEYCLOAK_VOTER_GROUP_NAME: ${KEYCLOAK_VOTER_GROUP_NAME}
      AWS_S3_PRIVATE_URI: ${AWS_S3_PRIVATE_URI}
      AWS_S3_PUBLIC_URI: ${AWS_S3_PUBLIC_URI}
      AWS_S3_BUCKET: ${AWS_S3_BUCKET}
      AWS_S3_PUBLIC_BUCKET: ${AWS_S3_PUBLIC_BUCKET}
      AWS_S3_ACCESS_KEY: ${AWS_S3_ACCESS_KEY}
      AWS_S3_ACCESS_SECRET: ${AWS_S3_ACCESS_SECRET}

      # used by AWS S3 to "load_from_env()". Don't use this in production, not
      # needed. Instead, in production we'll use Web Identity Tokens
      AWS_ACCESS_KEY_ID: ${AWS_S3_ACCESS_KEY}
      AWS_SECRET_ACCESS_KEY: ${AWS_S3_ACCESS_SECRET}

      SECRETS_BACKEND: ${SECRETS_BACKEND}
      AWS_SM_KEY_PREFIX: ${AWS_SM_KEY_PREFIX}
      VAULT_SERVER_URL: ${VAULT_SERVER_URL}
      VAULT_TOKEN: ${VAULT_TOKEN}

      AWS_S3_UPLOAD_EXPIRATION_SECS: ${AWS_S3_UPLOAD_EXPIRATION_SECS}
      AWS_S3_FETCH_EXPIRATION_SECS: ${AWS_S3_FETCH_EXPIRATION_SECS}
      AWS_REGION: ${AWS_REGION}
      AWS_S3_MAX_UPLOAD_BYTES: ${AWS_S3_MAX_UPLOAD_BYTES}

      CLOUDFLARE_ZONE: ${CLOUDFLARE_ZONE}
      CLOUDFLARE_API_KEY: ${CLOUDFLARE_API_KEY}
      CUSTOM_URLS_IP_DNS_CONTENT: ${CUSTOM_URLS_IP_DNS_CONTENT}
      B3_PG_HOST: ${B3_PG_HOST}
      B3_PG_PORT: ${B3_PG_PORT}
      B3_PG_USER: ${B3_PG_USER}
      B3_PG_PASSWORD: ${B3_PG_PASSWORD}
      B3_PG_DATABASE: ${B3_PG_DATABASE}
    ports:
     - ${HARVEST_PORT}:${HARVEST_PORT}

  b3:
    profiles: ["full", "base"]
    stdin_open: true
    image: 581718213778.dkr.ecr.us-east-1.amazonaws.com/b3:STEP_VERSION
    pull_policy: never
    container_name: b3
    restart: always
    ports:
      - "50051:50051"
    command: ["--host", "${B3_PG_HOST}", "--port", "${B3_PG_PORT}", "--username", "${B3_PG_USER}", "--password", "${B3_PG_PASSWORD}", "--database", "${B3_PG_DATABASE}", "--bind", "0.0.0.0:50051"]
    environment:
      RUSTFLAGS: ${RUSTFLAGS}
      RUST_BACKTRACE: ${RUST_BACKTRACE}
      LOG_LEVEL: ${LOG_LEVEL}
      CARGO_TERM_COLOR: ${CARGO_TERM_COLOR}
      B3_PG_HOST: ${B3_PG_HOST}
      B3_PG_PORT: ${B3_PG_PORT}
      B3_PG_USER: ${B3_PG_USER}
      B3_PG_PASSWORD: ${B3_PG_PASSWORD}
      B3_PG_DATABASE: ${B3_PG_DATABASE}
      B3_BIND: ${B3_BIND}

  trustee1:
    profiles: ["full"]
    stdin_open: true
    container_name: trustee1
    image: 581718213778.dkr.ecr.us-east-1.amazonaws.com/braid:STEP_VERSION
    pull_policy: never
    volumes:
      - ./trustees-data/trustee1/trustee1.toml:/opt/braid/trustee1.toml
    environment:
        TRUSTEE_NAME: trustee1
        TRUSTEE_CONFIG: ${TRUSTEE1_CONFIG}
        IGNORE_BOARDS: ${IGNORE_BOARDS}
        SECRETS_BACKEND: ${SECRETS_BACKEND}
        VAULT_SERVER_URL: ${VAULT_SERVER_URL}
        VAULT_TOKEN: ${VAULT_TOKEN}
        B3_URL: ${B3_URL}
    depends_on:
      immudb:
        condition: service_healthy

  trustee2:
    profiles: ["full"]
    stdin_open: true
    image: 581718213778.dkr.ecr.us-east-1.amazonaws.com/braid:STEP_VERSION
    pull_policy: never
    container_name: trustee2
    volumes:
      - ./trustees-data/trustee2/trustee2.toml:/opt/braid/trustee2.toml
    depends_on:
      immudb:
        condition: service_healthy
    environment:
        TRUSTEE_NAME: trustee2
        TRUSTEE_CONFIG: ${TRUSTEE2_CONFIG}
        IGNORE_BOARDS: ${IGNORE_BOARDS}
        SECRETS_BACKEND: ${SECRETS_BACKEND}
        VAULT_SERVER_URL: ${VAULT_SERVER_URL}
        VAULT_TOKEN: ${VAULT_TOKEN}
        B3_URL: ${B3_URL}

  trustee3:
    profiles: ["full"]
    stdin_open: true
    image: 581718213778.dkr.ecr.us-east-1.amazonaws.com/braid:STEP_VERSION
    pull_policy: never
    container_name: trustee3
    volumes:
      - ./trustees-data/trustee3/trustee3.toml:/opt/braid/trustee3.toml
    depends_on:
      immudb:
        condition: service_healthy
    environment:
        TRUSTEE_NAME: trustee3
        TRUSTEE_CONFIG: ${TRUSTEE3_CONFIG}
        IGNORE_BOARDS: ${IGNORE_BOARDS}
        SECRETS_BACKEND: ${SECRETS_BACKEND}
        VAULT_SERVER_URL: ${VAULT_SERVER_URL}
        VAULT_TOKEN: ${VAULT_TOKEN}
        B3_URL: ${B3_URL}

  # Create collection in immudb
  immudb-log-audit-init:
    profiles: ["full", "base"]
    image: ghcr.io/sequentech/step/immudb-log-audit:latest
    pull_policy: never
    container_name: immudb-log-audit-init
    command: >
      create sql pgaudit_hasura
      --parser pgauditjsonlog
      --log-level debug
      --immudb-user ${IMMUDB_USER}
      --immudb-password ${IMMUDB_PASSWORD}
      --immudb-host ${IMMUDB_HOST}
      --immudb-port ${IMMUDB_PORT}
      --immudb-database ${IMMUDB_LOGS_DB}
    depends_on:
      - postgres
      - immudb

  # Send audit logs to immudb
  immudb-log-audit:
    profiles: ["full", "base"]
    image: ghcr.io/sequentech/step/immudb-log-audit:latest
    pull_policy: never
    container_name: immudb-log-audit
    command: >
      tail file pgaudit_hasura "/logs/*.json"
      --follow
      --parser pgauditjsonlog
      --file-registry-dir=/data
      --log-level debug
      --immudb-user ${IMMUDB_USER}
      --immudb-password ${IMMUDB_PASSWORD}
      --immudb-host ${IMMUDB_HOST}
      --immudb-port ${IMMUDB_PORT}
      --immudb-database ${IMMUDB_LOGS_DB}
    volumes:
      - 'immudb_log_audit_data:/data'
      - 'db_logs:/logs'
    environment:
      IMMUDB_LOG_AUDIT_PROBE_ADDR: ${IMMUDB_LOG_AUDIT_PROBE_ADDR}
      IMMUDB_LOG_AUDIT_PROBE_LIVE_PATH: ${IMMUDB_LOG_AUDIT_PROBE_LIVE_PATH}

  # Create collection in immudb
  immudb-log-audit-init-keycloak:
    profiles: ["full", "base"]
    image: ghcr.io/sequentech/step/immudb-log-audit:latest
    pull_policy: never
    container_name: immudb-log-audit-init-keycloak
    command: >
      create sql pgaudit_keycloak
      --parser pgauditjsonlog
      --log-level debug
      --immudb-user ${IMMUDB_USER}
      --immudb-password ${IMMUDB_PASSWORD}
      --immudb-host ${IMMUDB_HOST}
      --immudb-port ${IMMUDB_PORT}
      --immudb-database ${IMMUDB_LOGS_DB}
    depends_on:
      - postgres
      - immudb
      - immudb-log-audit-init

  # Send audit logs to immudb
  immudb-log-audit-keycloak:
    profiles: ["full", "base"]
    image: ghcr.io/sequentech/step/immudb-log-audit:latest
    pull_policy: never
    container_name: immudb-log-audit-keycloak
    command: >
      tail file pgaudit_keycloak "/logs/*.json"
      --follow
      --parser pgauditjsonlog
      --file-registry-dir=/data
      --log-level debug
      --immudb-user ${IMMUDB_USER}
      --immudb-password ${IMMUDB_PASSWORD}
      --immudb-host ${IMMUDB_HOST}
      --immudb-port ${IMMUDB_PORT}
      --immudb-database ${IMMUDB_LOGS_DB}
    volumes:
      - 'keycloak_immudb_log_audit_data:/data'
      - 'keycloak_db_logs:/logs'

  rabbitmq:
    profiles: ["full", "base"]
    image: rabbitmq:3.12.11-management
    pull_policy: never
    container_name: rabbitmq
    environment:
      RABBITMQ_DEFAULT_USER: ${RABBITMQ_DEFAULT_USER}
      RABBITMQ_DEFAULT_PASS: ${RABBITMQ_DEFAULT_PASS}
    ports:
      - "5672:5672"
      - "15672:15672"
    healthcheck:
      test: rabbitmq-diagnostics -q ping
      interval: 30s
      timeout: 30s
      retries: 3

  immudb-init:
    profiles: ["full", "base"]
    stdin_open: true
    image: 581718213778.dkr.ecr.us-east-1.amazonaws.com/immudb-init:STEP_VERSION
    pull_policy: never
    container_name: immudb-init
    environment:
      IMMUDB_SERVER_URL: ${IMMUDB_SERVER_URL}
      IMMUDB_USER: ${IMMUDB_USER}
      IMMUDB_PASSWORD: ${IMMUDB_PASSWORD}
      IMMUDB_INDEX_DB: ${IMMUDB_INDEX_DB}
      IMMUDB_BOARD_DB_NAME: ${IMMUDB_BOARD_DB_NAME}
    volumes:
      - ./keycloak/import:/opt/keycloak/data/import:z
    restart: always

  windmill:
    profiles: ["full", "base"]
    stdin_open: true
    image: 581718213778.dkr.ecr.us-east-1.amazonaws.com/windmill:STEP_VERSION
    pull_policy: never
    volumes:
      - ./keycloak/import:/opt/keycloak/data/import:z
    restart: always
    environment:
      RUSTFLAGS: ${RUSTFLAGS}
      RUST_BACKTRACE: ${RUST_BACKTRACE}
      LOG_LEVEL: ${LOG_LEVEL}
      CARGO_TERM_COLOR: ${CARGO_TERM_COLOR}
      HASURA_ENDPOINT: ${HASURA_ENDPOINT}
      AMQP_ADDR: ${AMQP_ADDR}
      KEYCLOAK_URL: ${KEYCLOAK_URL}
      IMMUDB_SERVER_URL: ${IMMUDB_SERVER_URL}
      IMMUDB_USER: ${IMMUDB_USER}
      IMMUDB_PASSWORD: ${IMMUDB_PASSWORD}
      IMMUDB_INDEX_DB: ${IMMUDB_INDEX_DB}
      AWS_S3_PRIVATE_URI: ${AWS_S3_PRIVATE_URI}
      AWS_S3_PUBLIC_URI: ${AWS_S3_PUBLIC_URI}
      AWS_S3_BUCKET: ${AWS_S3_BUCKET}
      AWS_S3_PUBLIC_BUCKET: ${AWS_S3_PUBLIC_BUCKET}
      AWS_S3_ACCESS_KEY: ${AWS_S3_ACCESS_KEY}
      AWS_S3_ACCESS_SECRET: ${AWS_S3_ACCESS_SECRET}
      AWS_S3_JWKS_CACHE_POLICY: ${AWS_S3_JWKS_CACHE_POLICY}

      # used by AWS S3 to "load_from_env()". Don't use this in production, not
      # needed. Instead, in production we'll use Web Identity Tokens
      AWS_ACCESS_KEY_ID: ${AWS_S3_ACCESS_KEY}
      AWS_SECRET_ACCESS_KEY: ${AWS_S3_ACCESS_SECRET}

      AWS_S3_UPLOAD_EXPIRATION_SECS: ${AWS_S3_UPLOAD_EXPIRATION_SECS}
      AWS_S3_FETCH_EXPIRATION_SECS: ${AWS_S3_FETCH_EXPIRATION_SECS}
      AWS_REGION: ${AWS_REGION}
      AWS_S3_MAX_UPLOAD_BYTES: ${AWS_S3_MAX_UPLOAD_BYTES}

      SECRETS_BACKEND: ${SECRETS_BACKEND}
      AWS_SM_KEY_PREFIX: ${AWS_SM_KEY_PREFIX}
      VAULT_SERVER_URL: ${VAULT_SERVER_URL}
      VAULT_TOKEN: ${VAULT_TOKEN}

      KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH: ${KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH}
      KEYCLOAK_TENANT_REALM_CONFIG_PATH: ${KEYCLOAK_TENANT_REALM_CONFIG_PATH}
      KEYCLOAK_ADMIN_CLIENT_ID: ${KEYCLOAK_ADMIN_CLIENT_ID}
      KEYCLOAK_ADMIN_CLIENT_SECRET: ${KEYCLOAK_ADMIN_CLIENT_SECRET}
      KEYCLOAK_CLIENT_ID: ${KEYCLOAK_CLIENT_ID}
      KEYCLOAK_CLIENT_SECRET: ${KEYCLOAK_CLIENT_SECRET}
      SUPER_ADMIN_TENANT_ID: ${SUPER_ADMIN_TENANT_ID}
      VOTING_PORTAL_URL: ${VOTING_PORTAL_URL}
      SMS_TRANSPORT_NAME: ${SMS_TRANSPORT_NAME}
      AWS_SNS_ATTRIBUTES: ${AWS_SNS_ATTRIBUTES}
      EMAIL_TRANSPORT_NAME: ${EMAIL_TRANSPORT_NAME}
      EMAIL_FROM: ${EMAIL_FROM}
      KEYCLOAK_DB__USER: ${KEYCLOAK_DB__USER}
      KEYCLOAK_DB__PASSWORD: ${KEYCLOAK_DB__PASSWORD}
      KEYCLOAK_DB__HOST: ${KEYCLOAK_DB__HOST}
      KEYCLOAK_DB__PORT: ${KEYCLOAK_DB__PORT}
      KEYCLOAK_DB__DBNAME: ${KEYCLOAK_DB__DBNAME}
      KEYCLOAK_DB__MANAGER__RECYCLING_METHOD: ${KEYCLOAK_DB__MANAGER__RECYCLING_METHOD}
      HASURA_DB__USER: ${HASURA_DB__USER}
      HASURA_DB__PASSWORD: ${HASURA_DB__PASSWORD}
      HASURA_DB__HOST: ${HASURA_DB__HOST}
      HASURA_DB__PORT: ${HASURA_DB__PORT}
      HASURA_DB__DBNAME: ${HASURA_DB__DBNAME}
      HASURA_DB__MANAGER__RECYCLING_METHOD: ${HASURA_DB__MANAGER__RECYCLING_METHOD}
      LOW_SQL_LIMIT: ${LOW_SQL_LIMIT}
      DEFAULT_SQL_LIMIT: ${DEFAULT_SQL_LIMIT}
      DEFAULT_SQL_BATCH_SIZE: ${DEFAULT_SQL_BATCH_SIZE}
      WINDMILL_PROBE_ADDR: ${WINDMILL_PROBE_ADDR}
      WINDMILL_PROBE_LIVE_PATH: ${WINDMILL_PROBE_LIVE_PATH}
      WINDMILL_PROBE_READY_PATH: ${WINDMILL_PROBE_READY_PATH}

      PUBLIC_ASSETS_PATH: ${PUBLIC_ASSETS_PATH}
      PUBLIC_ASSETS_LOGO_IMG: ${PUBLIC_ASSETS_LOGO_IMG}
      PUBLIC_ASSETS_QRCODE_LIB: ${PUBLIC_ASSETS_QRCODE_LIB}
      PUBLIC_ASSETS_VOTE_RECEIPT_TEMPLATE: ${PUBLIC_ASSETS_VOTE_RECEIPT_TEMPLATE}
      PUBLIC_ASSETS_VOTE_RECEIPT_TEMPLATE_CONTENT: ${PUBLIC_ASSETS_VOTE_RECEIPT_TEMPLATE_CONTENT}
      PUBLIC_ASSETS_VELVET_VOTE_RECEIPTS_TEMPLATE: ${PUBLIC_ASSETS_VELVET_VOTE_RECEIPTS_TEMPLATE}
      PUBLIC_ASSETS_EML_BASE_TEMPLATE: ${PUBLIC_ASSETS_EML_BASE_TEMPLATE}
      VOTE_RECEIPT_TEMPLATE_TITLE: ${VOTE_RECEIPT_TEMPLATE_TITLE}
      VELVET_VOTE_RECEIPTS_TEMPLATE_TITLE: ${VELVET_VOTE_RECEIPTS_TEMPLATE_TITLE}
       #Demo key
      DEMO_PUBLIC_KEY: ${DEMO_PUBLIC_KEY}
      B3_PG_HOST: ${B3_PG_HOST}
      B3_PG_PORT: ${B3_PG_PORT}
      B3_PG_USER: ${B3_PG_USER}
      B3_PG_PASSWORD: ${B3_PG_PASSWORD}
      B3_PG_DATABASE: ${B3_PG_DATABASE}

  beat:
    profiles: ["full"]
    stdin_open: true
    image: 581718213778.dkr.ecr.us-east-1.amazonaws.com/windmill:STEP_VERSION
    pull_policy: never
    container_name: beat
    entrypoint: ["./beat"]
    depends_on:
      rabbitmq:
        condition: service_healthy
    environment:
      RUSTFLAGS: ${RUSTFLAGS}
      LOG_LEVEL: ${LOG_LEVEL}
      CARGO_TERM_COLOR: ${CARGO_TERM_COLOR}
      HASURA_ENDPOINT: ${HASURA_ENDPOINT}
      AMQP_ADDR: ${AMQP_ADDR}
      BEAT_PROBE_ADDR: ${BEAT_PROBE_ADDR}
      BEAT_PROBE_LIVE_PATH: ${BEAT_PROBE_LIVE_PATH}
      BEAT_PROBE_READY_PATH: ${BEAT_PROBE_READY_PATH}

volumes:
  db_data:
  db_logs:
  keycloak_db_logs:
  immudb_log_audit_data:
  keycloak_immudb_log_audit_data:
  keycloak_db_data:
  immudb_data:
  immudb_logs:
  minio_storage:
  protocol_manager_data:
  vault-volume:
EOF
    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL docker-compose.yml
}

add-keycloak-data-to-tarball() {
    tmpdir=$(mktemp -d)
    mkdir -p $tmpdir/keycloak
    cp -r $PROJECT_ROOT/.devcontainer/keycloak/import $tmpdir/keycloak
    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL keycloak
}

add-trustees-data-to-tarball() {
    tmpdir=$(mktemp -d)
    cp -r $PROJECT_ROOT/.devcontainer/trustees-data $tmpdir/trustees-data
    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL trustees-data
}

add-hasura-data-to-tarball() {
    tmpdir=$(mktemp -d)
    mkdir -p $tmpdir/hasura
    cp -r $PROJECT_ROOT/hasura/{metadata,migrations} $tmpdir/hasura
    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL hasura
}

add-up-script-to-tarball() {
    tmpdir=$(mktemp -d)
    cat <<'EOF' > $tmpdir/up
#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

echo "Cleaning up docker env"

docker ps -aq | xargs docker rm -f
docker images -q | xargs docker rmi -f
docker volume ls -q | xargs docker volume rm -f
docker network ls -q | xargs docker network rm -f
docker system prune -f

set -xeo pipefail

echo "Loading environment variables..."
source .env
echo "Creating resources..."
mkdir -p simplesaml
touch simplesaml/{authsources,saml20-sp-remote}.php
echo "Loading images..."
find images -type f -name "*.tar" | xargs -I{} docker load --input {}
echo "Starting environment..."
docker compose --profile full up
EOF
    chmod +x $tmpdir/up
    tar --append -C $tmpdir --file=$DELIVERABLE_TARBALL up
}

add-images-to-tarball() {
    tar --append -C $AIRGAPPED_ARTIFACTS_TODAY --file=$DELIVERABLE_TARBALL images
}

clean-artifacts-root() {
    rm -rf $AIRGAPPED_ARTIFACTS_TODAY
}

mkdir -p $DELIVERABLE_PATH $IMAGE_ARTIFACTS_PATH
tar -cf $DELIVERABLE_TARBALL -T /dev/null

build-images
pull-images

# Archive all images
for image in $(all-images); do
    archive-image-artifact "$image"
done

add-images-to-tarball
add-dotenv-to-tarball
add-docker-compose-to-tarball
add-keycloak-data-to-tarball
add-trustees-data-to-tarball
add-hasura-data-to-tarball
add-up-script-to-tarball
add-database-init-to-tarball
add-readme-to-tarball
add-janitor-to-tarball

clean-artifacts-root

info "Project root: $PROJECT_ROOT"
info "Air gapped artifacts location: $AIRGAPPED_ARTIFACTS_ROOT"
