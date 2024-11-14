<!--
SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only

-->

# Sequent Voting Platform

This is a mono-repo project encompasing the whole second generation of Sequent
Voting Platform. 

Implemented using:

- **Hasura** for GraphQL backend services API.
- **Rust** with **Rocket** for implementing custom backend services API logic.
- **Keycloak** as the IAM service.
- **PostgreSQL** for database storage for both Hasura and Keycloak.
- **React** for the frontend UI.
- Shared **Rust** libraries for logic shared by both frontend and
  backend.
- **Immudb** for tamper-evident logging.

## Development environment setup

Open the repository with devcontainers/codespaces within vscode. This will
launch all the services in development mode, so that you are ready to start
using them and continue development:

- **Keycloak** at [http://127.0.0.1:8090]:
  - `master` realm:
    - Username: `admin`
    - Password: `admin`
    - Telephone `+34666000222` (ends in `0222`)
    - Configure an OTP method the first time
  - election event realm (used through the react frontend for voting portal):
    - Username: `felix`
    - Password: `felix`
    - Telephone `+34666000111` (ends in `0111`)
    - Configure an OTP method the first time
- **Hasura console** at [http://127.0.0.1:8080].
  - This docker service has the `hasura/migrations` and `hasura/metadata`
    services mounted, so that you can work transparently on that and it's synced
    to and from the development environment.
- **React Frontend** at [http://127.0.0.1:3000].
  - This has the `packages/test-app` directory mounted in the docker service,
    and has been launched with the `yarn dev` command that will automatically
    detect and rebuild changes in the code, and detect and install dependencies
    when it detects changes in `package.json` and then relaunch the service.
- **Immudb**:
  - gRPC service available at [http://127.0.0.1:3322]
  - Web console at [http://127.0.0.1:3325]
  - Default admin
    - Username: `immudb`
    - Password: `immudb`
  - To create the index db, run:
    `/workspaces/step/packages/target/debug/bb_helper --cache-dir /tmp/cache -s http://immudb:3322 -i indexdb -u immudb -p immudb upsert-init-db -l debug`

Additionally, this dev container comes with:

- Relevant VS Code plugins installed
- `cargo run` and `yarn install` pre-run so that you don't have to spend time
  waiting for setting up the enviroment the first time.

## Developing `admin-portal`

To launch the `admin-portal` in development mode, execute (the first time):

```bash
cd /workspaces/step/packages/
yarn && yarn build:ui-core && yarn build:ui-essentials # only needed the first time
yarn start:admin-portal
```

For subsequent runs, you only need:

```bash
cd /workspaces/step/packages/
yarn start:admin-portal
```

Then it should open the admin-portal in the web browser, or else enter
in [http://127.0.0.1:3002/]

## Workspaces

When you open a new terminal, typically the current working directory (CWD) is
`/workspaces` if you are using Github Codespaces. However, all the commands
below are assuming you start with the CWD `/workspaces/step`.

This is important especially if you are for example relaunching a docker service
(for example `docker compose up -d graphql-engine`). If you do it from within
`/workspace/.devcontainer` it will fail, but if you do it within
`/workspaces/step/.devcontainer` it should work, even if those two
are typically a symlink to the other directory and are essentially the same.

## Directory tree file organization

The directory tree is structured as follows:

```bash
.
├── hasura                      <--- Hasura metadata and migrations in YAML
│   ├── metadata
│   └── migrations
├── packages                    <--- Main code of the application
│   ├── admin-portal
│   ├── braid
│   ├── harvest
│   ├── immu-board
│   ├── immudb-rs
│   ├── new-ballot-verifier
│   ├── sequent-core
│   ├── strand
│   ├── target
│   ├── test-app
│   ├── ui-essentials
│   └── voting-portal
└── vendor                      <--- External cloned dependencies
    └── immudb-log-audit
```

The `packages/` directory contains both `Cargo` and `Yarn` managed packages:
In that directory you can find both a `package.json` and a `Cargo.toml`. It's
at the same time a [cargo workspace] and a [yarn workspace].

This superimposed workspaces structure allows us to build the same module both
in yarn and cargo, depending on the use-case. For example, `sequent-core` is
both used in:
a. Frontend code, compiled to WASM with Yarn.
b. Backend code, compiled to native code with Cargo.

## Launch the backend rust service

Since we have not yet setup a docker container to automatically launch the
rust&rocket based backend service, you can launch it manually by executing the
following command in a dedicated terminal:

```bash
cd packages/harvest && cargo run
```

This should output something like:

```bash
@edulix ➜ /workspaces/step/packages/harvest (main ✗) $ cargo run
    Updating crates.io index
  Downloaded async-trait v0.1.68
  ....
  Downloaded 102 crates (7.9 MB) in 0.93s (largest was `encoding_rs` at 1.4 MB)
   Compiling harvest v0.1.0 (/workspace)
    Finished dev [unoptimized + debuginfo] target(s) in 28.50s
     Running `target/debug/harvest`
🔧 Configured for debug.
   >> address: 127.0.0.1
   >> port: 8000
   >> workers: 2
   >> max blocking threads: 512
   >> ident: Rocket
   >> IP header: X-Real-IP
   >> limits: bytes = 8KiB, data-form = 2MiB, file = 1MiB, form = 32KiB, json = 1MiB, msgpack = 1MiB, string = 8KiB
   >> temp dir: /tmp
   >> http/2: true
   >> keep-alive: 5s
   >> tls: disabled
   >> shutdown: ctrlc = true, force = true, signals = [SIGTERM], grace = 2s, mercy = 3s
   >> log level: normal
   >> cli colors: true
📬 Routes:
   >> (hello_world) GET /hello-world
📡 Fairings:
   >> Shield (liftoff, response, singleton)
🛡️ Shield:
   >> Permissions-Policy: interest-cohort=()
   >> X-Frame-Options: SAMEORIGIN
   >> X-Content-Type-Options: nosniff
🚀 Rocket has launched from http://127.0.0.1:8000
```

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

## Update `sequent-core`

```bash
cd /workspaces/step/packages/sequent-core
wasm-pack build --mode no-install --out-name index --release --target web --features=wasmtest
wasm-pack -v pack .
```

This returns a hash that you need to put in 3 different places in the  yarn.lock 
of packages/ directory:

```bash
"sequent-core@file:./admin-portal/rust/sequent-core-0.1.0.tgz":
  version "0.1.0"
  resolved "file:./admin-portal/rust/sequent-core-0.1.0.tgz#01a1bb936433ef529b9132c783437534db75f67d"

"sequent-core@file:./ballot-verifier/rust/sequent-core-0.1.0.tgz":
  version "0.1.0"
  resolved "file:./ballot-verifier/rust/pkg/sequent-core-0.1.0.tgz#01a1bb936433ef529b9132c783437534db75f67d"

"sequent-core@file:./voting-portal/rust/sequent-core-0.1.0.tgz":
  version "0.1.0"
  resolved "file:./voting-portal/rust/sequent-core-0.1.0.tgz#01a1bb936433ef529b9132c783437534db75f67d"
```

Then you need to execute some further updates:

```bash
cd /workspaces/step/packages/
rm ./ui-core/rust/sequent-core-0.1.0.tgz ./admin-portal/rust/sequent-core-0.1.0.tgz ./voting-portal/rust/sequent-core-0.1.0.tgz ./ballot-verifier/rust/sequent-core-0.1.0.tgz
cp sequent-core/pkg/sequent-core-0.1.0.tgz ./ui-core/rust/sequent-core-0.1.0.tgz
cp sequent-core/pkg/sequent-core-0.1.0.tgz ./admin-portal/rust/sequent-core-0.1.0.tgz
cp sequent-core/pkg/sequent-core-0.1.0.tgz ./voting-portal/rust/sequent-core-0.1.0.tgz
cp sequent-core/pkg/sequent-core-0.1.0.tgz ./ballot-verifier/rust/sequent-core-0.1.0.tgz

rm -rf node_modules ui-core/node_modules voting-portal/node_modules ballot-verifier/node_modules admin-portal/node_modules

yarn && yarn build:ui-core && yarn build:ui-essentials && yarn build:voting-portal && yarn build:admin-portal
```

And then everything should work and be updated. 

### Troubleshooting

If the typescript (TS, TSX) files suddently don't have correct autocompletion in
VSCode after this, the recommendation is to run the `Developer: Reload Window`
task in VSCode.

After running these commands, you need to stop any ui and relaunch. For some
reason craco is not going to be available, so you need run first
`Tasks: Run Task` > `start.build.admin-portal` which install it and all its
dependencies. Then you can launch also for example the `start.voting-portal`
task.

## Create election event

In order to be able to create an election event, you need:

1. Run harvest:

```bash
cd /workspaces/step/.devcontainer
docker compose down harvest && \              # stops & remove the container
docker compose up -d --no-deps harvest && \   # brings up the contaner
docker compose logs -f --tail 100 harvest     # tails the logs of the container
```

2. Run the vault:

```bash
cd /workspaces/step/.devcontainer
docker compose stop vault; docker compose up -d --no-deps vault
```

3. Go to `http://127.0.0.1:8201` and set 1 key (both fields), then note down the
   `Initial root token` and `Key 1` or `Download keys` in JSON. Then click in
   `Continue to Unseal`. Put `Key 1` (`keys[0]` in the downloaded keys) in
   `Unseal Key Portion` and press `Unseal`. If it works, it will redirect to
   `Sign in to Vault`. You can stop there.

4. We'll generate an `.env` file for windmill. Start copying the example:

```bash
cd /workspaces/step/packages/windmill
cp .env.example .env
```

5. Copy the `Initial root token` (`"root_token"` in the downloaded keys) to the
   `VAULT_TOKEN` environment variable in the
   `/workspaces/step/packages/windmill/.env` file.

6. Without windmill the async background tasks - like the creation of an
   election event - won't happen. For this reason, next we're going to run
   windmill:

```bash
cd /workspaces/step/packages/windmill
cargo run --bin main consume -q short_queue tally_queue beat reports_queue beat
```

7. Finally, we need to create the indexdb in immudb:

```bash
cd /workspaces/step/packages/immu-board
cargo build && \
../target/debug/bb_helper \
  --server-url http://immudb:3322 \
  --username immudb \
  --password immudb \
  --index-dbname indexdb \
  --board-dbname 33f18502a67c48538333a58630663559 \
  --cache-dir /tmp/immu-board upsert-init-db
```

Now you should be able to create election events. For debugging, you can watch the logs of `harvest` and `windmill` (it's already in one terminal):

```bash
# do this in one terminal
cd /workspaces/step/.devcontainer
docker compose logs -f harvest
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

## Tamper-evident logging

Here are some helpful random commands for development of the tamper-evident
logging implemented using immudb:

```bash
cd /workspaces/step/.devcontainer && docker compose build immudb-log-audit immudb-log-audit-init && docker compose up -d immudb-log-audit immudb-log-audit-init && docker compose logs -f immudb-log-audit
cd /workspaces/step/.devcontainer && docker compose build postgres && docker compose up -d postgres && docker compose logs -f postgres

docker compose exec postgres bash
docker compose run  --entrypoint /bin/sh immudb-log-audit

docker compose exec \
  -e PGPASSWORD=postgrespassword \
  postgres \
  psql \
  -h postgres \
  -U postgres

CREATE TABLE table1_with_pk (a SERIAL, b VARCHAR(30), c TIMESTAMP NOT NULL, PRIMARY KEY(a, c));
INSERT INTO table1_with_pk (b, c) VALUES('Backup and Restore', now()); 
```

### The disk/codespace runs out of space

Clean the disk with:

```bash
docker system prune --all --force
nix-collect-garbage
cargo clean
```

[cargo workspace]: https://doc.rust-lang.org/cargo/reference/workspaces.html
[yarn workspace]: https://yarnpkg.com/features/workspaces

## Public assets

### Vote receipt

The user can create a vote receipt in PDF from the confirmation screen on the `voting-portal`. To generate that PDF, we store some public assets on `minio`/`s3` at `public/public-asssets/*`.
Examples: 
- logo
- vendor to generate QR code
- HTML / HBS template
 
These assets are located here: `step/.devcontainer/minio/public-assets` and are uploaded to `minio` using the `configure-minio` container.

## Nightwatch e2e

### Running nightwatch(Admin-Portal)

Requires running both codespace instance as well as local instance at least for the client side.
 - run codespace
 - run local instance of client application to test
 - change directory to specific client application
 - npx nightwatch path/to/testfile.test.ts e.g `admin-portal% npx nightwatch test/e2e/voter.test.ts`
 
 ### Running Nightwatch(Voting-Portal)
 refer to voting-portal/test/readme
