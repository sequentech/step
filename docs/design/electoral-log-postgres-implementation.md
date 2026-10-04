<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# PostgreSQL electoral log implementation log

Parent issue: https://github.com/sequentech/meta/issues/13698

## Scope

Main/B4 only; breaking replacement with no historical ImmuDB migration. Use a dedicated electoral-log database on the existing PostgreSQL server. Preserve signed message encoding and the existing queue topology. Implementation and reviews cover this replacement, its configuration, documentation and focused correctness checks.

The subsequent retirement removes the obsolete pgAudit reader/UI and unused logEvent action, together with the ImmuDB crates, Dockerfiles, build jobs and Compose services. Historical investigation found that the UI was hidden in February 2024 and its unused collector was removed in November 2025. No historical data is copied; existing data volumes and backups are preserved.

## Implementation steps

1. Inspected the clean existing checkout on devenv-felix3 and refreshed origin/main to fc5cbe91ceb. Created feat/meta-13698/main. The developer started the Codespace containers through VS Code. Initial resources: 8 CPUs, 31 GiB RAM, 77 GiB free disk.
2. Created meta #13698, assigned Findeton, Iteration 154, status In progress, with backend/ImmuDB/breaking-change labels.
3. Inventoried the board client, direct list/count SQL, dispatcher, import/export and event lifecycle. Wrote focused PostgreSQL contract tests and confirmed they failed before implementation because the new domain/port/adapter API did not exist.
4. Implemented one storage port, a thin BoardClient service, parameterized PostgreSQL queries, a bounded shared connection pool, atomic iterator-based appends, stable delivery identities and serialized per-board writes before ID allocation.
5. Replaced electoral-log SQL in Windmill/Harvest callers with typed queries. Updated lifecycle, durable prepared entries, queue delivery identity, CLI generation, cast-vote export and the standalone generate-logs exporter. Imports preserve raw bytes and roll back the entire import on an error; exports page completely and propagate failures.
6. Added Compose initialization on the existing PostgreSQL service, cloud connection templates, AWS/GCP dedicated database/secrets/backup grants and operator documentation. Companion Beyond/GitOps branches are isolated. The runtime Beyond submodule pin and existing deployed environment values are unchanged. No cloud apply was performed.
7. Ran four PostgreSQL contract tests against a dedicated disposable database on the existing development server. Added a deterministic lock-timeout assertion within the existing concurrency test and timestamp-zero/order validation assertions within the existing query test, keeping the test count small.
8. Limited compilation to two jobs and capped Rust containers at two CPUs each. Paused watch supervisors while running tests against the shared target cache. Stopped the automatic duplicate CLI release build and reclaimed disposable Docker builder cache; no volumes or user data were removed.
9. Initialized the dedicated development electoral_log role/database and canonical schema with the new bootstrap script. Validated merged Compose configuration, changed YAML/rendered templates, Terraform syntax/formatting and whitespace. These are static infrastructure checks, not a cloud plan/apply.
10. Ran independent hostile reviews of storage, caller wiring, queue identity, authorization, exports, lifecycle and both companion repositories. Corrected an inaccurate stale-action route change and both retained Harvest macro dependencies. Follow-up review passed with no actionable findings. Reviewers performed no builds or mutations.

## Verification

Passed checks:

- Four actual-PostgreSQL contract tests: raw-row fidelity; scoped lifecycle/deletion; atomic rollback including streaming parse failure; concurrent retry idempotency; distinct identical-content events; serialized appends; typed filters and injection-as-data; zero timestamp; count/order/offset/cursor completeness; election/area/general visibility.
- Original delivery identity survives dispatch and task retry, with late acknowledgements and retry settings asserted.
- Prepared password-change entry round-trips through its durable JSON payload.
- Actual Windmill service wiring: prepared append/retry, list/count, raw signed bytes, tenant isolation and deletion.
- Eight targeted seeded faults all failed the tests; restored baseline passed. No timeouts, unviable mutants or survivors in this selected set.

| Seeded fault | Result |
| --- | --- |
| Remove board predicate from reads/counts | Killed |
| Make cursor boundary inclusive | Killed |
| Reverse cursor order | Killed |
| Remove per-board write lock | Killed |
| Roll back successful appends instead of committing | Killed |
| Remove idempotent conflict handling | Killed |
| Invert user-only restriction | Killed |
| Exclude the minimum timestamp boundary | Killed |

The selected fault score is 8/8 (100%). This is targeted mutation evidence for critical storage invariants, not an exhaustive cargo-mutants or repository-wide score. The tests run against PostgreSQL and catch SQL semantic faults that ordinary Rust operator mutation alone would miss. No additional tests were added solely to inflate this score.

Additional verification passed:

- Both existing CLI cast-vote export cases (legacy defaults and stored voting channel).
- The standalone generate-logs target compiles.
- All 11 existing electoral-log signed-message/encoding compatibility tests.
- Refreshed development PostgreSQL/API/worker containers retain existing volumes and use the new connection settings. The dedicated role successfully runs schema initialization and create/delete smoke operations. Re-running the initialization script succeeds without recreating roles/databases.
- Watch supervisors are restored. Windmill, Beat and Harvest compiled and are running. Harvest returns HTTP 401 for an unauthenticated POST to the new /electoral-log route.
- Removed only older completed task-era incremental-cache variants, keeping current application builds and all data volumes. Free disk recovered to approximately 29 GiB before the final Harvest build and remained approximately 25 GiB afterward.

Published PRs: Step #3420, Beyond #939 and GitOps #11138, all targeting main from feat/meta-13698/main. No production deployment or cloud apply has been performed.

## ImmuDB retirement follow-up

- Removed the unused ImmuDB dependency chain and narrowed all GraphQL snapshots/generated clients to the retired API fields and types.
- Kept PostgreSQL pgAudit session logging independent of the removed ImmuDB reader.
- Corrected main/B4 image paths, binary/features and offline configuration while removing the obsolete image jobs.
- Removed the development ImmuDB containers without deleting their volumes; restarted watch services with two CPUs and two Cargo jobs each.
- Beyond/GitOps deployment retirement remains a separate companion change. Older environments must upgrade before shared legacy infrastructure is retired.

## Trellis integration

- Imported `ruescasd/mrkl` branch `trellis` at `57ddd6d171ae6fc9f1545a1302f2d1e82f2defb7` into `packages/trellis`, retaining source provenance and the original tools behind an opt-in feature.
- Added a PostgreSQL journal with one generation per board and ordered leaves linked to source record IDs. Appends share the existing message transaction and delivery deduplication.
- Added versioned full-record hashing, checkpoint/inclusion/consistency operations and offline CLI verification. Existing message encoding, list queries and CSV exports remain unchanged.
- Embedded the bounded processor and proof API in Harvest, reusing the existing electoral-log database configuration and deployment. Each replica rebuilds and catches up independently, including when no new messages arrive.
- Updated workspace/image packaging and reused the existing PostgreSQL CI job. Added the imported tree tests to the Rust test matrix.
- Development work uses the requested SSH checkout. Watchers were stopped during editing; compilation is serialized with one Cargo job and one CPU per active build. The contract runner is capped at 8 GiB and uses the existing target directory. Existing data volumes are preserved.

Verification passed:

- Eight imported Trellis tree tests and 11 existing signed-message compatibility tests.
- Five PostgreSQL contracts, including the new combined Trellis append/proof/restart test. The final contract run used the current source and the workspace lockfile.
- CLI checkpoint, inclusion, consistency and offline verification against disposable records; modifying a bundled record makes verification fail.
- Live internal Harvest checkpoint/inclusion/consistency routes with synthetic test claims, automatic background catch-up, and rejection of requests without an authorization header. This exercises the internal API, not the external login flow.
- Actual Harvest process restart reconstructs the identical checkpoint and produces a valid consistency proof from the earlier saved checkpoint. Temporary fixture boards were deleted afterward.
- Formatting, whitespace, workspace metadata and changed-file REUSE metadata checks. The imported revision has no upstream license declaration; its provenance records that fact rather than assigning it a new license.

Harvest and Windmill watch builds completed under one-CPU limits. This follow-up does not include a complete production/offline image build, a historical-data migration or a full election lifecycle test. A user-initiated machine restart also exposed a stale PGMQ SQL mount from the other branch; the Keycloak PostgreSQL container was recreated from this branch's Compose definition with its existing data volume.

## Persisted Merkle tree, checkpoints and audits

Review of the Trellis integration found that every Harvest instance rebuilt and held each board's tree in memory, that lagging and diverged checkpoints were hard to tell apart, and that nothing outside the log database anchored its history. The journal now stores the tree:

- Storage: `trellis_nodes` holds every complete perfect subtree, written once by the append that completes it, and `trellis_logs` holds the size and root. An append extends the right edge read from the stored subtrees, in the transaction that stores the records. Proofs for any size read `O(log n)` subtrees with a few indexed queries from one snapshot. No instance keeps state or runs a background processor, so every instance returns the same checkpoint. The roots are those of the in-memory tree, so earlier checkpoints and bundles still verify.
- Integrity on every read: the stored right edge must produce the stored root before a checkpoint, proof or append is served. Stored data that is inconsistent is reported as such (HTTP 500), distinct from an unknown log or record (404) and from a checkpoint outside the log's history (409). Logs created before subtrees were stored are marked by the empty root and must be rebuilt with `backfill-nodes`; on other logs a rebuild only accepts a stored root that is the root of a prefix of the leaves.
- Published checkpoints: when voting closes (from a queued task, so the closure never waits for the publication) and when a results tally completes, Windmill signs the current checkpoint with the event's protocol-manager key, stores it in the Hasura database and records it in the log.
- Audits: an `AUDIT_ELECTORAL_LOG` task, started after each completed results tally and on demand from the Logs tab, the `audit_electoral_log` action or `cli step audit-electoral-log`, checks records against leaves, leaf order against record order, every stored subtree and the root against the leaves, and every published checkpoint's signer, signature and root against the roots recomputed from the leaves. Audits of a board run one at a time. Findings fail the task and are listed in its logs and, for tally audits, summarized in the tally logs. Audits never repair data.
- Verification against a trusted checkpoint: inclusion requests and the CLI accept a trusted checkpoint. The proof is computed at that checkpoint when it already contains the record, and otherwise at the current checkpoint together with a consistency proof from it.

The RFC 6962 arithmetic is checked byte for byte against `ct-merkle` for every root, inclusion proof and consistency proof of trees up to 130 leaves and for several larger trees. PostgreSQL tests cover atomic appends across insert chunks, concurrent appends, historical and anchored proofs, forged, future, other-generation and other-board checkpoints, tampered subtrees, roots, records and leaf order, legacy logs, refused rebuilds and concurrent audits. Windmill tests cover checkpoint signing against the audit's checks, and a PostgreSQL wiring test audits a board written through Windmill.
