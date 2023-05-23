<!--
SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Sequent backend-services

WARNING: This is a work-in-progress - not usable yet.

The backend-services for Sequent Voting Platform. This includes:
- admin-api
- ballot-box

Implemented using hasura and rust.

## Development environment setup

Open the repository with devcontainers/codespaces within vscode. Once that is 
done, you can:

1. **Launch the backend rust service** to provide the REST API with all the 
   complex logic that is not just reading from the database, by executing the
   following command in a dedicated terminal:

```bash
cargo run
```

This should output something like:

```bash
@edulix ➜ /workspaces/backend-services (main ✗) $ cargo run
    Updating crates.io index
  Downloaded async-trait v0.1.68
  ....
  Downloaded 102 crates (7.9 MB) in 0.93s (largest was `encoding_rs` at 1.4 MB)
   Compiling backend-services v0.1.0 (/workspaces/backend-services)
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

2. **Launch Hasura console**, to enable automatic tracking of the changes in the
   database metadata executing the following command in a dedicated terminal:

```bash
cd /workspace/hasura/ && hasura deploy && hasura console
```

This should have an output similar to:

```bash
@edulix ➜ /workspaces/backend-services (main ✗) $ cd hasura/
@edulix ➜ /workspaces/backend-services/hasura (main ✗) $ hasura console
NFO Help us improve Hasura! The cli collects anonymized usage stats which
allow us to keep improving Hasura at warp speed. To opt-out or read more,
visit https://hasura.io/docs/latest/graphql/core/guides/telemetry.html 
WARN Error opening browser, try to open the url manually?  error="exec: \"xdg-open\": executable file not found in $PATH"
INFO console running at: http://localhost:9695/
```

After that, you can open the hasura console at [http://localhost:9695/].
