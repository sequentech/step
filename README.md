<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only

-->

# Sequent Voting Platform

[![Chat][discord-badge]][discord-link]
[![Build Status][build-badge]][build-link]
[![License][license-badge]][license-link]
[![REUSE][reuse-badge]][reuse-link]
[![Documentation][docs-badge]][docs-link]

**Sequent Voting Platform** is an end-to-end verifiable, secure, and transparent
online voting system. This monorepo contains the second generation of the
platform, designed for real-world elections with strong cryptographic
guarantees.

## Key Features

- **End-to-end verifiability**: Voters can verify their vote was recorded, tallied, and counted correctly
- **Privacy-preserving**: Cryptographic mixnet ensures ballot secrecy while maintaining verifiability
- **Multi-tenant**: Support for multiple organizations and simultaneous elections
- **Accessible**: Web-based interfaces optimized for accessibility and usability
- **Auditable**: Comprehensive tamper-evident logging of all system operations

## Technology Stack

- **Hasura** - GraphQL API layer
- **Rust** - Cryptographic core and backend services
- **Keycloak** - Identity and access management
- **PostgreSQL** - Data persistence
- **React** - Frontend user interfaces
- **ImmuDB** - Tamper-evident audit logging

## Getting Started

For comprehensive documentation, visit [docs.sequentech.io](https://docs.sequentech.io/docusaurus/main/).

### Quick Start with Dev Containers

The fastest way to start developing is using VS Code Dev Containers or GitHub Codespaces:

1. Clone this repository
2. Open in VS Code with Dev Containers extension
3. All services will start automatically

The development environment includes:
- **Keycloak** at http://127.0.0.1:8090
- **Hasura Console** at http://127.0.0.1:8080
- **Voting Portal** at http://127.0.0.1:3000
- **Admin Portal** at http://127.0.0.1:3002
- **ImmuDB Console** at http://127.0.0.1:3325

For detailed setup instructions, see the [Developer Documentation](https://docs.sequentech.io/docusaurus/main/docs/developers/graphql-api).

### Manual Setup

If you prefer not to use Dev Containers:

1. Install prerequisites: Rust, Node.js/Yarn, Docker, PostgreSQL
2. Set up environment variables (see `.env.example`)
3. Start services: `docker compose up -d`
4. Run migrations: `cd hasura && hasura migrate apply`
5. Start frontend: `cd packages && yarn && yarn dev`

See the [documentation](https://docs.sequentech.io/docusaurus/main/) for detailed instructions.

## Repository Structure

```
.
├── .devcontainer/          # Dev container configuration
├── docs/docusaurus/        # Documentation site
├── hasura/                 # GraphQL schema, migrations, and metadata
├── packages/               # Monorepo packages (Cargo + Yarn workspace)
│   ├── admin-portal/       # Admin interface (React)
│   ├── voting-portal/      # Voter interface (React)
│   ├── braid/              # Cryptographic mixnet (Rust)
│   ├── sequent-core/       # Shared core libraries (Rust)
│   ├── ui-essentials/      # Shared UI components
│   └── ...                 # Other packages
└── README.md
```

The `packages/` directory is both a [Cargo workspace] and a [Yarn workspace], enabling code sharing between frontend (compiled to WebAssembly) and backend (native Rust).

## Contributing

We welcome contributions! Please see our [Contributing Guide](https://docs.sequentech.io/docusaurus/main/docs/developers/graphql-api) for details.

## License

This project is licensed under the AGPL-3.0-only license. See [LICENSE](LICENSE) for details.

## Support

- **Documentation**: https://docs.sequentech.io/docusaurus/main/
- **Discord**: Join our [Discord community][discord-link]
- **Issues**: Report bugs on [GitHub Issues](https://github.com/sequentech/step/issues)

---

## Developer Reference

For detailed developer documentation including advanced setup, architecture details, and troubleshooting, see the sections below or visit the [complete documentation](https://docs.sequentech.io/docusaurus/main/).

## Docker services logs

We have configured the use of [direnv] and [devenv] in this dev container, and
doing so in the `devenv.nix` file we configured the
`COMPOSE_PROJECT_NAME=step_devcontainer` env variable for
convenience and some utility packages automatically installed like `ack` or
`docker`.

Given that, you can then for example watch the log output of the frontend docker
compose service with:

`docker compose logs -f frontend`

And do the same for the other services. You could also do anything
docker-compose allows for example list running commands with`docker compose ps`.

With regards to the logs, we have configured in `.vscode/tasks.json` to
automatically run docker compose logs on start up, for convenience.

[direnv]: https://direnv.net/
[devenv]: https://devenv.sh/

## Immudb

You can enter the Immudb web console at http://localhost:3325 and the user/pass is `immudb:immudb`.

## Keycloak default realms

The deployment has 2 default Keycloak realms created by default, one for the
default tenant and another for the default election event inside that tenant.

Those two realms are automatically imported into Keycloak in the Dev Containers
from the `.devcontainer/keycloak/import/` directory.

Additionally, each tenant and election event have an associated realm. In the
Dev Containers, we use the same `.devcontainer/keycloak/import/` files to be the
templates for the creation of realms associated to a new tenant or a new
election event. These realms are created if they don't exist when the `keycloak`
container is started.

If you change the configuration of the default tenant realm and want to update
it in `.devcontainer/keycloak/import/` to be used for the default tenant and as
a template for new tenants, you can export it running the following commands:

```bash
export REALM="tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5"
cd /workspaces/step/.devcontainer
docker compose exec keycloak sh -c "/opt/keycloak/bin/kc.sh export --file /tmp/export.json --users same_file --realm ${REALM}"
docker compose exec keycloak sh -c 'cat /tmp/export.json' > keycloak/import/${REALM}.json
```

You can change `REALM` to be `"tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-33f18502-a67c-4853-8333-a58630663559"` to export and update the configuration of the default election event:

```bash
export REALM="tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-33f18502-a67c-4853-8333-a58630663559"
cd /workspaces/step/.devcontainer
docker compose exec keycloak sh -c "/opt/keycloak/bin/kc.sh export --file /tmp/export.json --users same_file --realm ${REALM}"
docker compose exec keycloak sh -c 'cat /tmp/export.json' > keycloak/import/${REALM}.json
```

Whenever a realm is updated, there's a chance that the assocated JWK used have
changed. This JWK is used to verify the JWT that is received from keycloak.
These keys are configured in S3/minio in the `.devcontainer/minio/certs.json`
file via the `configure-minio` helper docker service. If the keys changed and we
don't update the keys serviced by minio/s3, then the admin-portal or the
voting-booth might show some errors because this JWT verification fails.

To fix that issue by updating the JWK serviced by minio, perform the following
2 steps:

1. Update the `.devcontainer/minio/certs.json` file:

```bash
cd /workspaces/step/.devcontainer
[ -f /tmp/combined.json ] && rm /tmp/combined.json
export FILES=$(ls keycloak/import/)
for FILE in $FILES; do
  curl http://keycloak:8090/realms/${FILE%.json}/protocol/openid-connect/certs | python -m json.tool > /tmp/certs.json
  [ -f /tmp/combined.json ] && jq -s '{keys: (.[0].keys + .[1].keys)}' /tmp/certs.json /tmp/combined.json > /tmp/combined.json
  [ ! -f /tmp/combined.json ] && cp /tmp/certs.json /tmp/combined.json
done
ls -lah /tmp/certs.json /tmp/combined.json
cp /tmp/combined.json minio/certs.json
```

2. Rerun the `configure-minio` docker service to update the certificate serviced
   by `minio`:

```bash
cd /workspaces/step/.devcontainer/
docker compose build configure-minio && docker compose up -d --no-deps configure-minio && docker compose logs -f configure-minio
```

## Add Hasura migrations/changes

If you want to make changes to hasura, or if you want the Hasura console to
automatically add migrations to the code, first run this project in Codespaces
and open it in VS Code Desktop (not from the web). Then, in your local machine
ensure that the `graphql-engine` server name is aliased to `127.0.0.1` in
`/etc/hosts`, or else this won't work.

Then run the following commands to run the console in port `9695`:

```bash
cd /workspaces/step/hasura/
hasura console --endpoint "http://graphql-engine:8080" --admin-secret "admin"
```

Then open `http://localhost:9695` on the browser and make the changes you need.
Those changes will be tracked with file changes on the Github Codespaces, then
commit the changes.

Note that you can insert rows as a migration by clicking on the
`This is a migration` option at the bottom of the `Insert Row` form.

Note: if the browser doesn't load correctly at `http://localhost:9695`, try
opening the port `9693` in VS Code.

## admin-portal

## ui-essentials

Contains all the components used across the various portals i.e admin, voting, ballot etc.
Has storybook configured for component documentation and easy update of existing components or building new ones

To start storybook,
```bash
cd /workspaces/step/packages/
yarn storybook:ui-essentials
```

After updating any component in ui-essentials, run the following commands to build the current state.

```bash
cd /workspaces/step/packages/
yarn prettify:fix:ui-essentials && yarn build:ui-essentials
```

This is done to allow portals to fetch and use the latest versions of components

## Update graphql JSON schema

The file `packages/admin-portal/graphql.schema.json` contains the GraphQL/Hasura
schema. If the schema changes you might need to update this file. In order to do
so,
[follow this guide](https://hasura.io/docs/latest/schema/common-patterns/export-graphql-schema/)
to export the json schema from Hasura, specifically you'll need to run something
like:

```bash
cd /workspaces/step/packages/admin-portal/
gq http://graphql-engine:8080/v1/graphql \
    -H "X-Hasura-Admin-Secret: admin" \
    --introspect  \
    --format json \
    > graphql.schema.json
```

Afterwards, you need to regenerate the typescript auto-generated types using
`graphql-codegen` with:

```bash
cd /workspaces/step/packages/
yarn generate:admin-portal
```

Additionally, the same graphql schema file is needed in `windmill` to generate
the base types for Rust. To update them, execute the following:

```bash
cd /workspaces/step/packages/windmill/
gq http://graphql-engine:8080/v1/graphql \
    -H "X-Hasura-Admin-Secret: admin" \
    --introspect  \
    --format json \
    > src/graphql/schema.json
cargo build
```

It might be the case that for example if you added some new field to an existing
table, you will have to update some graphql query in
`packages/windmill/src/graphql/` directory and the corresponding boilerplate
code in `packages/windmill/src/hasura/`. Otherwise the build might fail.

## Creating Trustees

By default the trustees in this repo are configured to use a predefined configuration/
set of keys. This is useful for development because these trustees are also added to
the hasura/postgres database. This configuration is set using the `TRUSTEE_CONFIG`
environment paramenter in the docker-compose.yml file.

However if you want the trustees to generate their own unique public/private keys and
configuration this is is what you need to do:

First unset the `TRUSTEE_CONFIG` environment variable or set it to a file path that
doesn't exist. Then, when the trustee docker container is up, get the keys from the trustee:

```bash
docker exec -it trustee1 cat /opt/braid/trustee.toml | grep pk
```

Which will give a result similar to:

```bash
signing_key_pk = "YqYrRVXmPhBsWwwCgsOfw15RwUqZP9EhwmxuHKU5E8k"
```

Then add the trustee in the admin portal with the key, in this case `YqYrRVXmPhBsWwwCgsOfw15RwUqZP9EhwmxuHKU5E8k`.

## Running Trustees

```bash
# run windmill task generator
cd /workspaces/step/.devcontainer/
docker compose up -d beat && \
docker compose logs -f --tail 50 beat
```

```bash
# run trustes
cd /workspaces/step/.devcontainer/
docker compose up -d trustee1 trustee2 && \
docker compose logs -f --tail 50 trustee1 trustee2

```

## Vault

### HashiCorp Vault

We use HashiCorp Vault to store secrets. We run it in production mode as otherwise
the data would only be stored in memory and it would be lost each time the container
is restarted.

Once the `vault` container is started, you can log in here:

[http://127.0.0.1:8201/ui/vault/auth?with=token]

The first time you enter you'll have to note down the `initial root token` and the
`keys`. Then you need to enter that `key` (supposing you use only one key) to unseal
the vault and finally login with the `initial root token`.

Also in order for the `harvest` service to work, you'll first need to execute this:

    docker exec -it vault vault login

It will ask for the `initial root token`. This is required to authenticate for the
next step:

    docker exec -it vault vault secrets enable --version=1 --path=secrets kv

That will enable the /secrets path for the v1 key value secrets store in the `vault``.

You'll also need to configure the environment variables for `harvest` to connect
with the `vault`. Specifically, set the `VAULT_TOKEN` to the `initial root token`
and the `VAULT_UNSEAL_KEY` to the `keys`.

Finally you'll need to rebuild/restart harvest:

    docker compose stop harvest && docker compose build harvest && docker compose up -d --no-deps harvest

To configure project to use HashiCorpVault, setup the `VAULT_MANAGER` environment variable:

```
# .env
VAULT_MANAGER=HashiCorpVault
```

### AWS Secret Manager

To configure project to use AWS Secret Manager, setup the `VAULT_MANAGER` environment variable:

```bash
# .env
VAULT_MANAGER=AWSSecretManager
```

## Common issues

### Yarn build unexpectedly returns `exit code 143`

You run out of memory. Run in a bigger codespace or stop some service before
proceeding. You can stop for example the frontend with
`docker compose stop frontend`

### I changed my `Dockerfile` or my `docker-compose.yml`. Can I relaunch it without rebuilding the whole Dev Container?

Yes you can. For example, if you need to apply a new `DockerFile` o
`docker-compose.yml` config just for the frontend service, you can do the
following:

```bash
# First rebuild the service from the Dockerfile as indicated in the
# docker-compose.yml, then relaunch just the frontend service
docker compose build frontend && docker compose up -d --no-deps frontend
```

### The hasura schema changed or I want to add a query/mutation to the voting-portal/admin-portal

Add the query/mutation to the `packages/voting-portal/src/queries/` folder and
then run `yarn generate` from the `packages/` folder to update the types.
Similarly, run `yarn generate:admin-portal` to update the types of the
`admin-portal` if you need it.

### The voting portal will not load any elections

It's possible you find that the voting portal is not loading any elections, and
that inspecting it further, the Hasura/Graphql POST gives an error similar to
`field not found in type: 'query_root'`. This is possibly because you're
connecting to the wrong instance of Hasura. Possibly, you're running VS Code
with Codespaces and a local Hasura client as well, so the container port is
being forwarded to a different port than 8080.

## You get the `root does not exist` or hasura doesn't start, volumes don't mount

This is a nasty error that we need to further investigate and fix. Typically
start happening after the codespace has been stopped and restarted a few times.
Currently the only fix we have is.. committing and pushing all your changes to
your branch and starting a new codespace.


### The disk/codespace runs out of space

Clean the disk with:

```bash
docker system prune --all --force
nix-collect-garbage
cargo clean
```

### Fix starting/restarting containers

If you see an error when starting/restarting a container, remove the .docker folder:

```bash
rm -rf /home/vscode/.docker/
```

### Commiting with git

Unfortunately commiting with git doesn't work from the devcontainer. To commit to git,
ssh into the instance and cd into the step folder, then commit using git.


### Can't build sequent-core

If you're getting a permission error when building sequent-core, do:

```bash
sudo mkdir /workspaces/step/packages/target
sudo chown vscode:vscode /workspaces/step/packages/target -R
```


[discord-badge]: https://img.shields.io/discord/1006401206782001273?style=plastic
[discord-link]: https://discord.gg/WfvSTmcdY8

[build-badge]: https://github.com/sequentech/step/actions/workflows/pr_build.yml/badge.svg
[build-link]: https://github.com/sequentech/step/actions/workflows/pr_build.yml

[license-badge]: https://img.shields.io/github/license/sequentech/step?label=license
[license-link]: https://github.com/sequentech/step/blob/main/LICENSE

[reuse-badge]: https://api.reuse.software/badge/github.com/sequentech/step
[reuse-link]: https://api.reuse.software/info/github.com/sequentech/step

[docs-badge]: https://img.shields.io/badge/docs-docusaurus-blue
[docs-link]: https://docs.sequentech.io/docusaurus/main/

[cargo workspace]: https://doc.rust-lang.org/cargo/reference/workspaces.html
[yarn workspace]: https://yarnpkg.com/features/workspaces
