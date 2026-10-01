<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# PostgreSQL electoral log implementation log

Parent issue: https://github.com/sequentech/meta/issues/13698

## Scope

Main/B4 only; breaking replacement with no historical ImmuDB migration. Use a dedicated electoral-log database on the existing PostgreSQL server. Preserve signed message encoding and the existing queue topology. Implementation and reviews cover this replacement, its configuration, documentation and focused correctness checks.

The legacy pgAudit route and its external ingestion service are separate from electoral logging. They remain on ImmuDB pending an explicit decision to retire that API or port its producer/storage. This change does not claim complete removal of ImmuDB images or workspace packages while that consumer remains. No historical data is copied.

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

Companion local commits on feat/meta-13698/main: Beyond b7d2579a2; GitOps fbd3261. No branches have been pushed and no PR/deployment/cloud apply has been performed.
