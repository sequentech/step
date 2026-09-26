---
id: testing
title: Step CLI boundary tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Test file paths in this guide are relative to [`packages/step-cli/tests/`](https://github.com/sequentech/step/blob/main/packages/step-cli/tests). Commands state their working directory.

The credential-export slice covers independent PBKDF2 vectors, Unicode and
quoted CSV fields, preservation of previous files on malformed input, duplicate
password headers, reserved credential headers and actual subprocess exit codes.
All credentials are synthetic; tests use private temporary directories and low
iteration counts for fast feedback. Production defaults remain 600,000 iterations.

Credential conversion validates the complete CSV before publishing it. Duplicate
password headers and reserved credential headers are rejected. A successful
export atomically replaces the destination with a private file; a failed one
leaves the prior export intact. The CLI reports failures with a nonzero exit code.
Hashing processes at most 256 rows per batch, preserving source order without
retaining the entire plaintext census in memory. A failed-writer control counts
how many records were read; separate 600-row cases check batch order and cleanup
after a malformed final row.

Import tests compare file hashes with independent vectors, reject contradictory
sources before upload and refuse partial GraphQL results containing errors.

From the repository root:

```bash
python3 scripts/coverage/run.py step-cli --baseline --offline
```

Empty document references fail before network access; a valid reference remains
accepted. Input tests
pin the standard empty-file SHA-256, preserve I/O error types and reject trailing
JSON documents and invalid UTF-8 while retaining valid null and zero counts.
Command tests run the shipped import command against an owned loopback
HTTP peer with a private executable/config directory. Partial GraphQL data is
rejected before upload and after import, errors reach shell callers through a
nonzero exit, and empty error lists remain valid. Existing and dangling output
symlinks are rejected without changing either the link or its target. Each rejected input has a successful control.

Voter generation rules live in `src/domain/generate_voters.rs` and take the
random number generator and the current date as arguments, so
`support/voter_generation_boundaries.rs` uses seeded or scripted generators and
a fixed date. `support/voter_csv_boundaries.rs` writes the CSV to memory and to
a private directory. A failed `generate-voters` run prints the error and still
exits with status 0; `command_failures.rs` pins that. From `packages/`:

```bash
cargo test -p step-cli --bin step-cli generate_voters
cargo test -p step-cli --test command_failures
```

Tally-sheet commands reach Hasura through the `GraphqlClient` port and store
import sources through the `Uploader` and `Downloader` ports (`src/ports`).
`support/tally_sheet_commands.rs` runs each command against the in-memory
adapters in `src/adapters/memory`, which queue GraphQL responses and record
requests, uploads and downloads. It pins the request variables, the UUID checks
that `import-list`, `import-show`, `import-download-source` and `recount` make
before sending anything, GraphQL errors winning over returned data, the
download file name and the import source rules. `tally_sheet_cli.rs` runs the
shipped commands against a loopback server: failures print the error, including
the `HTTP Status` and `Error Message` of a rejected request, and only
`import-preview` and `import-create` exit with status 1. From `packages/`:

```bash
cargo test -p step-cli --bin step-cli tally_sheet
cargo test -p step-cli --test tally_sheet_cli
```

The telephone automation contract tests run without a deployment:

```bash
# Repository root; Python standard library only.
python3 -m unittest discover -s packages/step-cli/scripts -p 'test_*.py'
# The janitor CSV generator uses its pinned Faker dependency; PostgreSQL is not
# contacted by this test (the unrelated database import is isolated).
python3 -m venv /tmp/step-cli-python
/tmp/step-cli-python/bin/pip install Faker==13.3.4
/tmp/step-cli-python/bin/python -m unittest discover -s packages/windmill/external-bin/janitor/scripts -p 'test_voter_keys.py'
# Shipped CLI process and native contracts:
cd packages && cargo test --locked -p step-cli
```

Setup writes `tenants.json` as tenant IDs are returned and a provisional event
summary as soon as import completes. Cleanup can therefore recover recorded
resources after later provisioning fails, including tenants whose event summary
is absent. Its event-only, new-tenant-only and bootstrap protections still apply.
Each Python CLI subprocess has a 600-second default timeout; callers may pass a
shorter `timeout` to `run_step`. A timeout raises `StepCliError` for the existing
retry policy. Remote tasks already submitted may continue after the process ends.

`upload-document` requests a public storage endpoint by default; use `--is-local`
only when the CLI can reach the deployment's internal storage endpoint. Loopback
HTTP tests check the complete upload variables, exact signed PUT URL and bytes.
Refresh-token tests cover omitted, empty and rotated refresh tokens: a fresh
access token never discards the stored refresh token unless a nonempty replacement
is returned.

Voter CSV authorization values match the configured Keycloak mapper: use each
assigned election's nonempty `external_id`, falling back to `id` for missing,
null or empty external IDs. Display aliases serve country/embassy lookups only.
An area without assigned elections retains the explicit `Unknown` sentinel;
blank authorization attributes can invoke broader mapper fallback behavior.
Rust and janitor tests read actual CSV and cover mixed identifiers, duplicate
contest assignments and an area without elections. The legacy E2E civil-registry
CSV is not an authorization fixture: its mock loader ignores that column.

Kubernetes workers require a cluster supporting
[`backoffLimitPerIndex`](https://kubernetes.io/docs/concepts/workloads/controllers/job/#backoff-limit-per-index).
Each index has zero retries; failed indexes do not terminate remaining workers.
The coordinator polls for `Complete` or `Failed`, bounded by `execution.wait_timeout`,
and collects available results before returning a failure. Its process test uses
a local `kubectl` peer to exercise the real manifest, timeout, collection and
cleanup paths; it does not validate a cluster's scheduling or storage driver.
