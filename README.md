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