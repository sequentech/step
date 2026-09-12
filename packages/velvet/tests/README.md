<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Velvet boundary tests

Use small elections with totals that can be checked by hand. A passing test
should establish what is counted, rejected or published, including the failure
cases. These tests use local temporary files, in-memory SQLite and synthetic
ballots. They require no election credentials or external services.

| File | Contract exercised |
| --- | --- |
| `plurality_boundaries.rs` | Area weights apply to candidate marks; participation remains a count of ballots. Blank, invalid and declined ballots stay distinct. Candidate suppression preserves participation, and unknown candidate IDs fail. |
| `results_boundaries.rs` | Area aggregation preserves totals and percentages. Winner selection uses counts, tie ordering and eligible candidates. File input and output preserve area separation and report failures. |
| `database_boundaries.rs` | Real SQLite writes preserve identities and percentage denominators. A failed result generation rolls back the new event and schema changes. Decoded ballot files retain their exact bytes and area keys. |
| `pipeline_boundaries.rs` | Configuration requires defined stages and unique transformations. Empty stages and malformed folder names cannot panic. CLI locations are explicit. |
| `paper_pipeline.rs` | Paper batch totals, parent/child areas, consolidated reports, real PDF artifacts and command-line success/failure. |
| `support/report_boundaries.rs` | Custom candidate and winner ordering, preservation of the randomized snapshot, template errors and real browser PDF rejection. |
| `support/participation_boundaries.rs` | Participation and channel totals reject integer overflow. |
| `support/ballot_image_boundaries.rs` | Receipt conversion, candidate ordering, QR content, HTML escaping, CSV quoting, real PDF batches and manifests. Invalid templates and zero batch sizes fail the pipe. Compiled inside the receipt module to exercise private helpers without expanding the public API. |

The existing unit and integration suites also exercise IRV, ballot decoding,
report generation, local PDF rendering and complete pipeline execution. Two
pipeline fixtures use two elections, two contests, three areas and all 20
ballot cases. This retains the multi-election, multi-contest and missing-area
checks while avoiding hundreds of duplicate PDF renderings. Throughput and
large-election benchmarks are separate from this correctness suite.

For a fast edit/check cycle, from `packages` run:

```sh
cargo test -p velvet --locked --offline \
  --test plurality_boundaries --test results_boundaries \
  --test database_boundaries --test pipeline_boundaries
cargo test -p velvet --locked --offline --test paper_pipeline
cargo test -p velvet --locked --offline --lib -- boundary_tests
```

For the full native coverage report, install Chrome and the pinned coverage
tools, fetch locked dependencies, then run from the repository root:

```sh
google-chrome --version
RUST_TEST_THREADS=2 RAYON_NUM_THREADS=2 DOC_RENDERER_BACKEND=inplace \
  python3 scripts/coverage/run.py velvet --baseline --offline
```

The full suite uses a real local browser; an absent browser is an environment
failure, not a reason to skip PDF tests. The baseline command reports progress
toward 95% without claiming that threshold is met. Omit `--baseline` to enforce
the target. Current results and remaining work are tracked in Meta #13294.

The report retains all measured source lines and explains files without LLVM
line regions. Existing inline tests and public fixture builders contribute to
the native aggregate. This is not production-only coverage or branch coverage.
Cloud rendering, remote storage, ACM Java signing and deployment integrations
need their own local fixture recipes before their coverage can be claimed.
