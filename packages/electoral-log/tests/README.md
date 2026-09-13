<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Electoral Log tests

From the repository root, run the tests that need no database:

```bash
cd packages
cargo test -p electoral-log --locked
```

For the complete profile, install [ImmuDB 1.9.6](https://github.com/codenotary/immudb/releases/tag/v1.9.6),
the version used by this repository. Put `immudb` on PATH, or set
`ELECTORAL_LOG_TEST_IMMUDB_BINARY` to the executable's absolute path. Then run:

```bash
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
73-test suite passes with the owned ImmuDB 1.9.6 fixture; the one legacy fixed-port
test remains ignored. No production or coverage-exclusion changes were needed.

At `29e334a`, the complete database profile measures 1,343/1,378 lines (97.46%),
182/192 functions (94.79%) and 1,327/1,390 LLVM regions (95.47%). The actual PR
base has no `immudb-tests` feature, so that full profile has no comparable base.
The separate `electoral-log-native` profile measures default features on both
revisions for the strict per-metric CI comparison. It does not claim database
integration coverage. Neither missing feature support nor an empty report is
converted into a zero baseline.

Sorting on nonunique timestamps or metadata appends `id ASC` unless callers
already specify the ID direction. A literal SQL regression and real tied-row
pagination controls check both default and explicit tie-breaking. The default
suite now has 68 passing tests; the full profile has 73, including five owned
ImmuDB scenarios. No database ordering is inferred from insertion luck.
