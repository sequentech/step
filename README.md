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

### Launch the backend rust service

Since we have not yet setup a docker container to automatically launch the
rust&rocket based backend service, you can launch it manually by executing the
following command in a dedicated terminal:

```bash
cd backend/backend-services && cargo run
```

This should output something like:

```bash
@edulix ➜ /workspace/backend/backend-services (main ✗) $ cargo run
    Updating crates.io index
  Downloaded async-trait v0.1.68
  ....
  Downloaded 102 crates (7.9 MB) in 0.93s (largest was `encoding_rs` at 1.4 MB)
   Compiling backend-services v0.1.0 (/workspace)
    Finished dev [unoptimized + debuginfo] target(s) in 28.50s
     Running `target/debug/backend-services`
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
install the hasura client:

    curl -L https://github.com/hasura/graphql-engine/raw/stable/cli/get.sh | bash

Also clone this github project on your local machine (so this is apart from running
it on Codespaces), and from the `backend-services/hasura` folder, run this:

    hasura console --endpoint "http://127.0.0.1:8080" --admin-secret "admin"
  
Then open `http://localhost:9695` on the browser and make the changes you need.
Those changes will be tracked with file changes on your local github, then
commit the changes.

Note that you can insert rows as a migration by clicking on the 
`This is a migration` option at the bottom of the `Insert Row` form.

#### Or do it inside the codespace

Alternatively you could run the local console inside the `graphql-engine container`.
For that you need to add ports `9693` and `9695` to `forwardPorts` in the file
`.devcontainer/devcontainer.json` and add them to `graphql-engine.ports` in
file `.devcontainer/docker-compose.yml`.

You'll need to rebuild the container:

    docker compose stop graphql-engine
    docker compose build graphql-engine  && docker compose up -d --no-deps graphql-engine

Then you get inside the container with:

    docker compose exec graphql-engine /bin/sh

And run the hasura console with something like:

    /usr/bin/hasura-cli console --endpoint "http://127.0.0.1:8080" --admin-secret "admin" --address 0.0.0.0 --console-hge-endpoint http://127.0.0.1:8080

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

### The disk/codespace runs out of space

Clean the disk with:

    docker system prune --all
    nix-collect-garbage
