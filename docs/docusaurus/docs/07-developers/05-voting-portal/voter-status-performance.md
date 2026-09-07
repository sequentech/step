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

Run these commands from the repository root inside `devenv shell`, including inside a devcontainer. The stack needs the S3 voting path, a running portal, Hasura, Keycloak, object storage, cast services and configured trustees. Configure **step-cli** for a synthetic tenant using the [CLI guide](../02-cli/01-cli_cli.md). The setup command uses automatic trustees already registered in that tenant.

```bash
umask 077
mkdir -p .cache/voting-scale
cp scripts/voting_e2e/scale.example.json .cache/voting-scale/input.json
read -rs -p 'Shared synthetic voter password: ' LOAD_PASSWORD
export LOAD_PASSWORD

# Edit input.json: tenant, reachable URLs, voter count, concurrency and goals.
python3 scripts/voting_e2e/scale_setup.py \
  .cache/voting-scale/input.json .cache/voting-scale/setup \
  --cli /absolute/path/to/step-cli \
  --template packages/step-cli/scripts/telephone-load-test-inputs/election-event.json \
  --is-local
```

Setup imports an isolated one-election fixture, configures username lookup, exports the assigned IDs, imports the census in bounded batches, completes an automatic key ceremony, publishes and opens online voting. `setup/config.json` contains the resulting worker configuration. If the deployment prepares S3 files separately, add `--publication-preparer /absolute/path/to/prepare_ballot_files` from the application build, with its database and S3 environment configured. This invokes the application's locked, verified publication writer. Setup progress and errors stay in private logs; `setup-state.json` preserves created IDs if provisioning stops.

The census uses `username_prefix + index` and one shared password. CSVs contain `hashed_password`, `password_salt` and `num_of_iterations`, **never a `password` column**: that column would make the importer hash every row again. Match `hash_iterations` to the realm's PBKDF2-SHA256 policy. Login still verifies each password normally. Use exact username lookup; shared attribute-only matching cannot identify these voters efficiently.

For an already prepared event, provide its IDs, realm, area and login URL in your configuration, then run `scale.py census DIRECTORY --config CONFIG` and `scale.py import DIRECTORY --config CONFIG --cli CLI [--is-local]`.

## Run k6 or Chromium

Build the native encryption executable once for k6 voting. It streams fresh ciphertexts and proofs using the same library and signing policies as the portal; reusing the same selection does not reuse ballot IDs.

```bash
(cd packages && CARGO_TARGET_DIR=sequent-core/rust-local-target \
  cargo build --release -p sequent-core --example prepare_load_ballots \
  --features default_features)

python3 scripts/voting_e2e/scale.py prepare .cache/voting-scale/run \
  --config .cache/voting-scale/setup/config.json --nodes 4 \
  --native packages/sequent-core/rust-local-target/release/examples/prepare_load_ballots
python3 scripts/voting_e2e/scale.py run .cache/voting-scale/run --nodes 4
```

Preparation authenticates one census voter and downloads the current publication without casting. For the simple fixture it selects enough candidates to satisfy each contest; supply `--choices decoded-choices.json` for a different ballot. Encrypted inputs are ready before load timing begins. Republishing requires fresh preparation.

For full UI load, set `"engine": "chromium"` before preparation and omit `--native`. Chromium creates a new browser context per voter and performs selection, encryption and confirmation itself. Install the matching Playwright Chromium runtime, or use the Chromium worker image below. Use a fresh census range for each voting run.

`--nodes` controls independent workers; `vus` controls concurrent voters per worker. The finite workload runs until every assigned voter has attempted once or the k6 shard's `max_duration` expires. It measures achieved throughput under fixed concurrency, not a prescribed arrival rate. Increase nodes and concurrency progressively; goals fail on missing journeys, errors, duplicate receipts, excessive p50/p99 or insufficient casts/s.

## Containers and pods

A million-voter run with `shard_size: 10000` creates 100 input shards. Each worker loads one shard at a time. A claim is created atomically per shard; retries and replacement pods cannot silently cast that shard again. A failed or interrupted shard needs reconciliation, not deletion of its claim. The result merge uses disk-backed SQLite and computes global percentiles from individual samples.

```bash
bash scripts/voting_e2e/image.sh k6 registry.example/voting-load:k6
# Or build the full browser worker:
bash scripts/voting_e2e/image.sh chromium registry.example/voting-load:chromium

python3 scripts/voting_e2e/scale.py containers /absolute/path/to/run \
  --nodes 4 --image registry.example/voting-load:k6 --network YOUR_NETWORK
python3 scripts/voting_e2e/scale.py report /absolute/path/to/run
```

For Kubernetes, copy the prepared run to the root of a ReadWriteMany PVC and put the voter password in a Secret named `voting-load-password`, key `password`. The image contains source only. Neither administrator credentials nor census CSVs need to be mounted on casting workers.

```bash
python3 scripts/voting_e2e/scale.py pods /absolute/path/to/run \
  --nodes 20 --image registry.example/voting-load:k6 --pvc voting-load-run \
  > .cache/voting-scale/job.json
kubectl create -f .cache/voting-scale/job.json
```

The generated [indexed Job](https://kubernetes.io/docs/concepts/workloads/controllers/job/) partitions shards by completion index. The Job uses the coordinator UID/GID by default (`--uid`/`--gid` override these); ensure the prepared PVC files are readable and its result directory writable by that user. Workers must share the same claim directory on a filesystem supporting exclusive file creation. Resource requests/limits are starting values; size them from measured worker CPU, memory and network use. Preparation can also use multiple native processes with `prepare --nodes X`. It does not create one Kubernetes object per voter.

## Target a production deployment

Use an isolated synthetic election in the target deployment and configure step-cli against its tenant. Replace **every** origin in the input, including Keycloak and the object-store/CDN origins returned in signed URLs. Omit `--is-local` for production uploads. Use externally reachable HTTPS URLs, the deployment's CA trust and its configured portal URL (the importer derives client redirects from the deployment settings); `localhost` inside a pod refers to that pod.

Run setup and native preparation on a coordinator, distribute the prepared run, then launch workers near the intended client region. Keep the voter password in an environment variable or cluster Secret. Begin with a small cohort and raise `count`, `vus` and `--nodes` while observing both clients and services. The harness bounds client memory; backend capacity still depends on Keycloak, Hasura, database, cast processing and network resources. Census import and encryption are separate preparation costs, not included in casts/s.

## Read and refresh results

Every local run generates `report.html`, `performance.svg`, `performance.md` and `results.json`. The standalone HTML embeds its graphs and includes global p50/p99, accepted casts/s, request counts, GraphQL operation definitions and goal failures. Open it directly or print it to PDF. Raw samples and receipts remain private beside the report.

```bash
# Rebuild the report after collecting all remote result directories:
python3 scripts/voting_e2e/scale.py report /absolute/path/to/run --publish-docs
```

k6 records counts by endpoint/operation; `trace_http: true` additionally records each protocol fetch in private worker logs, with URL queries and authorization omitted. Chromium records GraphQL operations and resource paths. Add `report --dsn-env LOAD_AUDIT_DSN` for an independent PostgreSQL receipt audit using a read-only observer connection and batches of 1,000 IDs. The DSN remains on the coordinator. Reports distinguish API receipts from this audit; client traffic alone does not expose internal Hasura SQL. Use the [diagnostic capture](./voting-flow-e2e.md) with database observers when SQL attribution is needed.

<!-- scale-performance:start -->
Measured example: **k6**, 600/600 successful journeys, **51.57 accepted casts/s**. Voter-status p50/p99: **39.00/143.03 ms**.

Cast p50/p99: **32.00/114.26 ms**.

This is a measured workload, not a deployment capacity guarantee.

![Voting performance](/img/voting-scale-performance.svg)
<!-- scale-performance:end -->
