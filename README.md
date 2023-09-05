<!--
SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only

-->

# Sequent Voting Platform

WARNING: This is a work-in-progress - not usable yet.

This is a mono-repo project encompasing the whole second generation of Sequent
Voting Platform.

Implemented using:
- **Hasura** for GraphQL backend services API.
- **Rust** with **Rocket** for implementing custom backend services API logic.
- **Keycloak** as the IAM service.
- **PostgreSQL** for database storage for both Hasura and Keycloak.
- **React** for the frontend UI.
- \[TODO\] Shared **Rust** libraries for logic shared by both frontend and
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
  - `electoral-process` realm (used through the react frontend):
    - Username: `edu`
    - Password: `edu`
- **Hasura console** at [http://127.0.0.1:8080].
  - This docker service has the `hasura/migrations` and `hasura/metadata`
  services mounted, so that you can work transparently on that and it's synced
  to and from the development environment.
- **React Frontend** at [http://127.0.0.1:3000].
  - This has the `frontend/test-app` directory mounted in the docker service,
  and has been launched with the `yarn dev` command that will automatically
  detect and rebuild changes in the code, and detect and install dependencies
  when it detects changes in `package.json` and then relaunch the service.
- **Immudb**:
  - gRPC service available at [http://127.0.0.1:3322]
  - Web console at [http://127.0.0.1:3325]
  - Default admin
    - Username: `immudb`
    - Password: `immudb`
- \[TODO\] **Rust Rocket service** at [http://127.0.0.1:8000]

Additionally, this dev container comes with:
 - Relevant VS Code plugins installed
 - `cargo run` and `yarn install` pre-run so that you don't have to spend time
   waiting for setting up the enviroment the first time.

### Workspaces

When you open a new terminal, typically the current working directory (CWD) is
`/workspaces` if you are using Github Codespaces. However, all the commands
below are assuming you start with the CWD `/workspaces/backend-services`.

This is important especially if you are for example relaunching a docker service
(for example `docker compose up -d graphql-engine`). If you do it from within
`/workspace/.devcontainer` it will fail, but if you do it within
`/workspaces/backend-services/.devcontainer` it should work, even if those two
are typically a symlink to the other directory and are essentially the same.

### Launch the backend rust service

Since we have not yet setup a docker container to automatically launch the
rust&rocket based backend service, you can launch it manually by executing the
following command in a dedicated terminal:

```bash
cd backend/harvest && cargo run
```

This should output something like:

```bash
@edulix ➜ /workspaces/backend-services/backend/harvest (main ✗) $ cargo run
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

### Docker services logs

We have configured the use of [direnv] and [devenv] in this dev container, and
doing so in the `devenv.nix` file we configured the 
`COMPOSE_PROJECT_NAME=backend-services_devcontainer` env variable for 
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

### Export keycloak realm with users

If you want to export a realm configuration but you don't need the users,
you can do it from the realm console, going to the `Realm settings` and
clicking on `Action` > `Partial export`.

However that won't export users. You can export them by running this:

    docker compose exec keycloak sh -c '/opt/keycloak/bin/kc.sh export --file /tmp/export.json --users same_file --realm electoral-process'
    docker compose exec keycloak sh -c 'cat /tmp/export.json' > file.json

Then you'll find the export -including users- in the `file.json`. You
can then for example update the file `.devcontainer/keycloak/import/electoral-process-realm.json`
if you want to automatically import that data when the container is
created.

### Add Hasura migrations/changes

If you want to make changes to hasura, or if you want the Hasura console to
automatically add migrations to the code, first run this project in Codespaces
and open it in VS Code Desktop (not from the web). Then, in your local machine
ensure that the `graphql-engine` server name is aliased to `127.0.0.1` in 
`/etc/hosts`, or else this won't work.

Also clone this github project on your local machine (so this is apart from running
it on Codespaces), and from the `backend-services/hasura` folder, run this:

    hasura console
  
Then open `http://localhost:9695` on the browser and make the changes you need.
Those changes will be tracked with file changes on the Github Codespaces, then
commit the changes.

Note that you can insert rows as a migration by clicking on the 
`This is a migration` option at the bottom of the `Insert Row` form.

## templates
### Update graphql JSON schema

The file `backend/templates/src/graphql/schema.json` contains the GraphQL/Hasura schema. If the schema changes you might need to update this file. In order to do so, [follow this guide](https://hasura.io/docs/latest/schema/common-patterns/export-graphql-schema/
) to export the json schema from Hasura, specifically you'll need to run something like:

    npm install -g graphqurl
    gq http://127.0.0.1:8080/v1/graphql -H "X-Hasura-Admin-Secret: admin" --introspect  --format json > schema.json

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

### The hasura schema changed or I want to add a query/mutation to the voting-portal

Add the query/mutation to the `frontend/voting-portal/src/queries/` folder and 
then run `yarn generate` from the `frontend/` folder to update the types.

### The voting portal will not load any elections

It's possible you find that the voting portal is not loading any elections,
and that inspecting it further, the Hasura/Graphql POST gives an error similar to
`field not found in type: 'query_root'`. This is possibly because you're connecting
to the wrong instance of Hasura. Possibly, you're running VS Code with Codespaces
and a local Hasura client as well, so the container port is being forwarded to
a different port than 8080.

## Tamper-evident logging

Here are some helpful random commands for development of the tamper-evident 
logging implemented using immudb:

```bash
cd /workspaces/backend-services/.devcontainer && docker compose build immudb-log-audit immudb-log-audit-init && docker compose up -d immudb-log-audit immudb-log-audit-init && docker compose logs -f immudb-log-audit
cd /workspaces/backend-services/.devcontainer && docker compose build postgres && docker compose up -d postgres && docker compose logs -f postgres

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

    docker system prune --all
    nix-collect-garbage
