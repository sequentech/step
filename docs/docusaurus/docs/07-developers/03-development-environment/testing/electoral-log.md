---
id: electoral-log
title: Electoral Log tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Test file paths in this guide are relative to [`packages/electoral-log/tests/`](https://github.com/sequentech/step/blob/feat/meta-13302-ui-essentials-coverage/main/packages/electoral-log/tests). Commands state their working directory.

From the repository root, run the tests that need no database:

```bash
cd packages
cargo test -p electoral-log --locked
```

For the complete profile, install [ImmuDB 1.9.6](https://github.com/codenotary/immudb/releases/tag/v1.9.6),
the version used by this repository. Put `immudb` on PATH, or set
`ELECTORAL_LOG_TEST_IMMUDB_BINARY` to the executable's absolute path. From the repository root, run:

```bash
cd packages
cargo test -p electoral-log --features immudb-tests --locked
cd ..
python3 scripts/coverage/run.py electoral-log
```

The strict coverage command requires 95% package lines and writes HTML, LCOV,
JSON, compiler/profile details and test logs under `coverage/electoral-log/`.
Use `--baseline` when measuring unfinished work; it does not certify the target.
Run a focused test file while editing, then use the complete command for review.

Each database test starts its own ImmuDB process on a local ephemeral port with
synthetic credentials and a temporary data directory. It does not accept a
server URL or deployment credentials. The child is stopped and reaped even if a
test fails. The database scenarios have a two-minute deadline; they fail if the
binary is missing. The legacy ignored fixed-port test is preserved, but the new
suite covers its behavior without requiring an existing database.

The suite covers:

- Real sender/system signatures, altered statements, JSON/Borsh interchange,
  fixed wire tags, malformed encodings, every truncated message prefix and
  writers that fail at each payload boundary.
- Required and optional database fields, reordered columns, nulls, duplicate or
  missing fields, invalid labels and inconsistent row dimensions.
- ImmuDB schema creation, insertion, nullable metadata, failed transactions,
  filtering, epoch-zero bounds, sorting and both pagination strategies across
  903 records. The helper CLI also runs against the owned database.
- Stream failures after valid rows: a failed or malformed response is an error,
  never a successful partial audit page.

The existing protocol signs the statement. Search metadata and the separate
artifact are not authenticated by `Message::verify`; these tests make no broader
claim. Full election workflows, server persistence under power loss and native
branch coverage require separate evidence.

All unit tests live outside `src`, including the preserved existing tests, so
their bodies do not inflate source coverage. Derive-generated methods remain in
LLVM's measured totals. The initial 28.40% baseline included inline test bodies;
its denominator differs from this profile and is retained as historical evidence.

Follow-up row contracts give every optional field a distinct value, including an
empty ballot ID, so accidental field swaps or normalization are observable. A
valid explicit null must not hide a duplicate column from a joined table. The
74-test suite passes with the owned ImmuDB 1.9.6 fixture; the one legacy fixed-port
test remains ignored. Production fixes validate database column labels and order tied timestamps by ID.
Coverage exclusions are unchanged.

At `c4f9dc3`, the complete database profile measures 1,348/1,383 lines (97.47%),
182/192 functions (94.79%) and 1,338/1,401 LLVM regions (95.50%). The actual PR
base has no `immudb-tests` feature, so that full profile has no comparable base.
The separate `electoral-log-native` profile measures default features on both
revisions for the strict per-metric CI comparison. It does not claim database
integration coverage. Neither missing feature support nor an empty report is
converted into a zero baseline.

Sorting on nonunique timestamps or metadata appends `id ASC` unless callers
already specify the ID direction. A literal SQL regression and real tied-row
pagination controls check both default and explicit tie-breaking. The default
suite now has 68 passing tests; the full profile has 74, including six owned
ImmuDB scenarios. No database ordering is inferred from insertion luck.

The comparable default-feature run at `c4f9dc3` measures 1,079/1,383 lines
(78.02%), 140/192 functions (72.92%) and 1,109/1,401 regions (79.16%). Its
actual base has 10 tests and 461/1,623 lines, 28/201 functions and 473/1,675
regions. Every measured fraction increases; source inventories remain complete.

The hosted database job uses ImmuDB 1.9.6 with a unique synthetic password for
each owned process. Startup retries only address-in-use failures, at most five
times; a successful connection must authenticate to the owned process. Tests
cover a forced port collision, all 903 offset-paginated records, an actual epoch
zero row with an inclusive zero upper bound, and a full-capacity writer control.
