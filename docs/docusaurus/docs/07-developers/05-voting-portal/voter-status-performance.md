---
sidebar_position: 7
---

# Voting performance

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Choose **k6** to measure the authenticated voting API at scale, or **Chromium** to measure the complete voting portal. Both use distinct synthetic voters, real Keycloak authentication and the normal cast API. k6 needs no browser capture, browser installation or browser-based encryption.

| Stage | k6 | Chromium |
| --- | --- | --- |
| Login | Fresh cookies, authorization code + PKCE, username/password, token exchange | Portal redirect and Keycloak login form |
| List elections | `GetVoterStatus`, then S3 event, election and ballot summary | Render the election list from those responses |
| Select and encrypt | Native `sequent-core` encryption before timing | Select candidates and encrypt with portal WASM during timing |
| Cast | Submit the iteration's prepared ciphertext through `InsertCastVote` | Review, confirm, submit and display the ballot receipt |
| Resources | Authenticated protocol and four publication downloads | Also JavaScript, WASM, CSS, fonts and images |

`GetVoterStatus` is the minimal GraphQL read: signed publication URLs, eligibility and prior cast status. Election and ballot-style content comes from S3. Set `"mode": "status"` with k6 to measure login plus this API without downloading publications or casting; set only `status_ms` goals for that workload.

## Prepare an election from the command line

Run these commands from the repository root inside `devenv shell`, including inside a devcontainer. The stack needs the S3 voting path, a running portal, Hasura, Keycloak, object storage, cast services and configured trustees. Use a tenant reserved for synthetic voters. The commands below authenticate **step-cli** against that tenant. The setup command uses automatic trustees already registered in that tenant.

Use the same shell for configuration, setup and execution. Enter your tenant ID and CLI path when prompted; the URLs below are for the local devcontainer stack.

```bash
umask 077
mkdir -p .cache/voting-scale
read -r -p 'Absolute path to step-cli: ' STEP_CLI
read -r -p 'Synthetic tenant ID: ' LOAD_TENANT_ID
export LOAD_TENANT_ID
export LOAD_PORTAL_URL='http://localhost:3000'
export LOAD_KEYCLOAK_URL='http://keycloak:8090'
export LOAD_GRAPHQL_URL='http://graphql-engine:8080/v1/graphql'
export LOAD_STORAGE_ORIGIN='http://minio-proxy:9002'
read -rs -p 'Shared synthetic voter password: ' LOAD_PASSWORD
export LOAD_PASSWORD
```

Authenticate the tenant administrator. Use the client ID and secret configured for CLI access in that tenant's administrative realm. For a public client, leave the secret empty.

```bash
read -r -p 'Administrator username: ' LOAD_ADMIN_USER
read -rs -p 'Administrator password: ' LOAD_ADMIN_PASSWORD
read -r -p 'CLI client ID: ' LOAD_CLIENT_ID
read -rs -p 'CLI client secret: ' LOAD_CLIENT_SECRET

"$STEP_CLI" step config \
  --tenant-id "$LOAD_TENANT_ID" \
  --endpoint-url "$LOAD_GRAPHQL_URL" \
  --keycloak-url "$LOAD_KEYCLOAK_URL" \
  --keycloak-user "$LOAD_ADMIN_USER" \
  --keycloak-password "$LOAD_ADMIN_PASSWORD" \
  --keycloak-client-id "$LOAD_CLIENT_ID" \
  --keycloak-client-secret "$LOAD_CLIENT_SECRET"
unset LOAD_ADMIN_PASSWORD LOAD_CLIENT_SECRET
```

Create a 100-voter smoke-test configuration. `allowed_origins` contains only scheme, hostname and optional port, with no path or trailing slash. It limits the hosts the runner may contact. Include the storage/CDN host that your deployment uses when signing publication URLs; do **not** edit signed URLs themselves.

```bash
python3 - <<'PYTHON'
import json
import os
from pathlib import Path
from urllib.parse import urlsplit

config = json.loads(Path("scripts/voting_e2e/scale.example.json").read_text())
config.update(
    tenant_id=os.environ["LOAD_TENANT_ID"],
    portal_url=os.environ["LOAD_PORTAL_URL"],
    keycloak_url=os.environ["LOAD_KEYCLOAK_URL"],
    graphql_url=os.environ["LOAD_GRAPHQL_URL"],
    count=100,
    shard_size=25,
    vus=2,
    goals={"status_ms": {"p50": 100, "p99": 500}},
)
urls = [config[key] for key in ("portal_url", "keycloak_url", "graphql_url")]
urls.append(os.environ["LOAD_STORAGE_ORIGIN"])
config["allowed_origins"] = sorted({
    f"{urlsplit(url).scheme}://{urlsplit(url).netloc}" for url in urls
})
Path(".cache/voting-scale/input.json").write_text(json.dumps(config, indent=2) + "\n")
PYTHON
```

For the local devcontainer stack, provision the election with this command. For a remote deployment, use the [production setup command](#target-a-production-deployment) instead.

```bash
python3 scripts/voting_e2e/scale_setup.py \
  .cache/voting-scale/input.json \
  .cache/voting-scale/setup \
  --cli "$STEP_CLI" \
  --template packages/step-cli/scripts/telephone-load-test-inputs/election-event.json \
  --is-local
```

The CLI writes its session to `config/configuration.json` beside its executable; that directory must be writable.

Setup imports an isolated one-election fixture, configures username lookup, exports the assigned IDs, imports the census in bounded batches, completes an automatic key ceremony, publishes and opens online voting. `setup/config.json` contains the resulting worker configuration. Setup progress and errors stay in private logs; `setup-state.json` preserves created IDs if provisioning stops.

The census uses `username_prefix + index` and one shared password. CSVs contain `hashed_password`, `password_salt` and `num_of_iterations`, **never a `password` column**: that column would make the importer hash every row again. Match `hash_iterations` to the realm's PBKDF2-SHA256 policy. Login still verifies each password normally. Use exact username lookup; shared attribute-only matching cannot identify these voters efficiently.

For an existing event, reuse its generated `setup/config.json` with an unused voter range. The Chromium example below includes the commands to generate and import that range.

## Run k6 or Chromium

Build the native encryption executable once for k6 voting. It streams fresh ciphertexts and proofs using the same library and signing policies as the portal; reusing the same selection does not reuse ballot IDs.

```bash
(
  cd packages
  CARGO_TARGET_DIR=sequent-core/rust-local-target cargo build \
    --release \
    -p sequent-core \
    --example prepare_load_ballots \
    --features default_features
)

python3 scripts/voting_e2e/scale.py prepare .cache/voting-scale/run \
  --config .cache/voting-scale/setup/config.json \
  --nodes 4 \
  --native packages/sequent-core/rust-local-target/release/examples/prepare_load_ballots
python3 scripts/voting_e2e/scale.py run .cache/voting-scale/run \
  --nodes 4
```

Preparation authenticates one census voter and downloads the current publication without casting. For the supplied simple fixture it selects enough candidates to satisfy each contest. Encrypted inputs are ready before load timing begins. Republishing requires fresh preparation.

For full UI load, the following commands create a Chromium configuration with the next unused voter range, import its census, then prepare and run. Each browser context selects, encrypts and confirms its own ballot.

```bash
python3 - <<'PYTHON'
import json
from pathlib import Path

config = json.loads(Path(".cache/voting-scale/setup/config.json").read_text())
config["engine"] = "chromium"
config["start"] += config["count"]
Path(".cache/voting-scale/next-config.json").write_text(json.dumps(config, indent=2) + "\n")
PYTHON

python3 scripts/voting_e2e/scale.py census .cache/voting-scale/next-census \
  --config .cache/voting-scale/next-config.json
python3 scripts/voting_e2e/scale.py import .cache/voting-scale/next-census \
  --config .cache/voting-scale/next-config.json \
  --cli "$STEP_CLI" \
  --is-local
(
  cd packages/voting-portal
  yarn playwright install chromium
)
python3 scripts/voting_e2e/scale.py prepare .cache/voting-scale/chromium-run \
  --config .cache/voting-scale/next-config.json \
  --nodes 4
python3 scripts/voting_e2e/scale.py run .cache/voting-scale/chromium-run \
  --nodes 4
```

`--nodes` controls independent workers; `vus` controls concurrent voters per worker. The finite workload runs until every assigned voter has attempted once or the k6 shard's `max_duration` expires. It measures achieved throughput under fixed concurrency, not a prescribed arrival rate. Increase nodes and concurrency progressively; goals fail on missing journeys, errors, duplicate receipts, excessive p50/p99 or insufficient casts/s.

## Containers and pods

A million-voter run with `shard_size: 10000` creates 100 input shards. Each worker loads one shard at a time. A claim is created atomically per shard; retries and replacement pods cannot silently cast that shard again. A failed or interrupted shard needs reconciliation, not deletion of its claim. The result merge uses disk-backed SQLite and computes global percentiles from individual samples.

Docker and Kubernetes are alternatives to the local `run` command. Prepare a fresh range and run directory for each alternative; a directory already attempted locally cannot be cast again.

For Docker on the devcontainer network:

```bash
bash scripts/voting_e2e/image.sh k6 voting-load:k6
# Use this image with a configuration whose engine is chromium:
bash scripts/voting_e2e/image.sh chromium voting-load:chromium

read -r -p 'Absolute path to a fresh prepared run: ' LOAD_RUN
python3 scripts/voting_e2e/scale.py containers "$LOAD_RUN" \
  --nodes 4 \
  --image voting-load:k6 \
  --network step_devcontainer_default
python3 scripts/voting_e2e/scale.py report "$LOAD_RUN"
```

For Kubernetes, use a prepared run targeting the externally reachable deployment URLs described below. These commands use your current kubectl context and namespace. They require a storage class supporting ReadWriteMany and a registry accessible to your cluster. Choose PVC capacity large enough for prepared ballots and results.

```bash
read -r -p 'Worker image (registry.example.org/team/voting-load:k6): ' LOAD_IMAGE
read -r -p 'ReadWriteMany storage class: ' LOAD_STORAGE_CLASS
read -r -p 'PVC capacity (for example 20Gi): ' LOAD_STORAGE_SIZE
read -r -p 'Absolute path to a fresh prepared run: ' LOAD_RUN
bash scripts/voting_e2e/image.sh k6 "$LOAD_IMAGE"
docker push "$LOAD_IMAGE"

kubectl create secret generic voting-load-password \
  --from-file=password=<(printf '%s' "$LOAD_PASSWORD")
kubectl create \
  -f - <<YAML
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: voting-load-run
spec:
  accessModes: [ReadWriteMany]
  storageClassName: "$LOAD_STORAGE_CLASS"
  resources:
    requests:
      storage: "$LOAD_STORAGE_SIZE"
---
apiVersion: v1
kind: Pod
metadata:
  name: voting-load-transfer
spec:
  securityContext:
    runAsUser: $(id -u)
    runAsGroup: $(id -g)
    fsGroup: $(id -g)
  containers:
    - name: transfer
      image: "$LOAD_IMAGE"
      command: ["sleep", "86400"]
      volumeMounts:
        - name: load
          mountPath: /load
  volumes:
    - name: load
      persistentVolumeClaim:
        claimName: voting-load-run
YAML
kubectl wait pod/voting-load-transfer \
  --for=condition=Ready \
  --timeout=120s
kubectl cp "$LOAD_RUN/." voting-load-transfer:/load

python3 scripts/voting_e2e/scale.py pods "$LOAD_RUN" \
  --nodes 20 \
  --image "$LOAD_IMAGE" \
  --pvc voting-load-run \
  > .cache/voting-scale/job.json
kubectl create \
  -f .cache/voting-scale/job.json
```

Collect results after the Job completes (or fails); the report exposes missing or failed journeys too:

```bash
LOAD_JOB=$(python3 - <<'PYTHON'
import json
from pathlib import Path
print(json.loads(Path(".cache/voting-scale/job.json").read_text())["metadata"]["name"])
PYTHON
)
kubectl wait "job/$LOAD_JOB" \
  --for=condition=Complete \
  --timeout=1h
kubectl cp voting-load-transfer:/load/. "$LOAD_RUN"
python3 scripts/voting_e2e/scale.py report "$LOAD_RUN"
kubectl delete pod voting-load-transfer
```

Only the prepared run and synthetic voter password go to workers. Administrator credentials and census CSVs stay on the coordinator. Keep the PVC until results have been collected and any failed attempts reconciled.

The generated [indexed Job](https://kubernetes.io/docs/concepts/workloads/controllers/job/) partitions shards by completion index. The Job uses the coordinator UID/GID by default (`--uid`/`--gid` override these); ensure the prepared PVC files are readable and its result directory writable by that user. Workers must share the same claim directory on a filesystem supporting exclusive file creation. Resource requests/limits are starting values; size them from measured worker CPU, memory and network use. The `--nodes` option on the preparation commands also controls native encryption processes. It does not create one Kubernetes object per voter.

## Target a production deployment

Run from a coordinator that can reach the target deployment. Set these values **before** running the administrator authentication and configuration-generation blocks above. The example domains illustrate the four addresses you need; enter your deployment's actual addresses at the prompts.

```bash
read -r -p 'Synthetic production tenant ID: ' LOAD_TENANT_ID
read -r -p 'Portal URL (https://vote.example.org): ' LOAD_PORTAL_URL
read -r -p 'Keycloak URL (https://auth.example.org): ' LOAD_KEYCLOAK_URL
read -r -p 'GraphQL URL (https://api.example.org/v1/graphql): ' LOAD_GRAPHQL_URL
read -r -p 'Publication storage origin (https://ballots.example.org): ' LOAD_STORAGE_ORIGIN
export LOAD_TENANT_ID LOAD_PORTAL_URL LOAD_KEYCLOAK_URL LOAD_GRAPHQL_URL LOAD_STORAGE_ORIGIN
```

The storage address is the public S3/CDN origin configured by the deployment operator for publication downloads, not its internal bucket endpoint. If publication URLs use multiple storage hosts, include each host in `allowed_origins`. Workers must be able to resolve and connect to all these addresses and trust their TLS certificates. The portal URL must also match the deployment's configured voting-portal URL, which controls Keycloak redirects.

After generating `input.json` with those values, use this production setup command. It uploads through the server's signed storage URLs. The local-only `--is-local` option rewrites upload hosts for the devcontainer and is intentionally absent here.

```bash
python3 scripts/voting_e2e/scale_setup.py \
  .cache/voting-scale/input.json \
  .cache/voting-scale/production-setup \
  --cli "$STEP_CLI" \
  --template packages/step-cli/scripts/telephone-load-test-inputs/election-event.json

python3 scripts/voting_e2e/scale.py prepare .cache/voting-scale/production-run \
  --config .cache/voting-scale/production-setup/config.json \
  --nodes 4 \
  --native packages/sequent-core/rust-local-target/release/examples/prepare_load_ballots
python3 scripts/voting_e2e/scale.py run .cache/voting-scale/production-run \
  --nodes 4
python3 scripts/voting_e2e/scale.py report .cache/voting-scale/production-run
```

For a later census import against production, use the same import command without local host rewriting:

```bash
python3 scripts/voting_e2e/scale.py import .cache/voting-scale/next-census \
  --config .cache/voting-scale/next-config.json \
  --cli "$STEP_CLI"
```

Run setup and native preparation on a coordinator, distribute the prepared run, then launch workers near the intended client region. Keep the voter password in an environment variable or cluster Secret. Begin with a small cohort and raise `count`, `vus` and `--nodes` while observing both clients and services. The harness bounds client memory; backend capacity still depends on Keycloak, Hasura, database, cast processing and network resources. Census import and encryption are separate preparation costs, not included in casts/s.

## Read and refresh results

Every local run generates `report.html`, `performance.svg`, `performance.md` and `results.json`. The standalone HTML embeds its graphs and includes global p50/p99, accepted casts/s, request counts, GraphQL operation definitions and goal failures. Open it directly or print it to PDF. Raw samples and receipts remain private beside the report.

```bash
# Rebuild the report after collecting all remote result directories:
python3 scripts/voting_e2e/scale.py report .cache/voting-scale/run \
  --publish-docs
```

k6 records counts by endpoint/operation; `trace_http: true` additionally records each protocol fetch in private worker logs, with URL queries and authorization omitted. Chromium records GraphQL operations and resource paths. For an independent PostgreSQL receipt audit, supply a read-only observer connection. The audit reads receipt IDs in batches of 1,000:

```bash
read -rs -p 'Read-only backend PostgreSQL DSN: ' LOAD_AUDIT_DSN
export LOAD_AUDIT_DSN
python3 scripts/voting_e2e/scale.py report .cache/voting-scale/run \
  --dsn-env LOAD_AUDIT_DSN
unset LOAD_AUDIT_DSN
```
 The DSN remains on the coordinator. Reports distinguish API receipts from this audit; client traffic alone does not expose internal Hasura SQL. Use the [diagnostic capture](./voting-flow-e2e.md) with database observers when SQL attribution is needed.

<!-- scale-performance:start -->
Measured example: **k6**, 600/600 successful journeys, **51.57 accepted casts/s**. Voter-status p50/p99: **39.00/143.03 ms**.

Cast p50/p99: **32.00/114.26 ms**.

This is a measured workload, not a deployment capacity guarantee.

![Voting performance](/img/voting-scale-performance.svg)
<!-- scale-performance:end -->
