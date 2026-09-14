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
LLVM's measured totals.

Follow-up row contracts give every optional field a distinct value, including an
empty ballot ID, so accidental field swaps or normalization are observable. A
valid explicit null must not hide a duplicate column from a joined table. The separate `electoral-log-native` profile measures default features on both
revisions for the strict per-metric CI comparison. Use it when the actual base
does not provide the `immudb-tests` feature; retain full-profile evidence separately. It does not claim database
integration coverage. Neither missing feature support nor an empty report is
converted into a zero baseline.

Sorting on nonunique timestamps or metadata appends `id ASC` unless callers
already specify the ID direction. A literal SQL regression and real tied-row
pagination controls check both default and explicit tie-breaking. No database ordering is inferred from insertion luck.
