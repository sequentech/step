---
id: test-coverage
title: Velvet boundary tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Test file paths in this guide are relative to [`packages/velvet/tests/`](https://github.com/sequentech/step/blob/feat/meta-13302-ui-essentials-coverage/main/packages/velvet/tests). Commands state their working directory.

Use small elections with totals that can be checked by hand. A passing test
should establish what is counted, rejected or published, including the failure
cases. These tests use local temporary files, in-memory SQLite and synthetic
ballots. They require no election credentials or external services.

| File | Contract exercised |
| --- | --- |
| `plurality_boundaries.rs` | Area weights apply to candidate marks; participation remains a count of ballots. Blank, invalid and declined ballots stay distinct. Candidate suppression preserves participation, and unknown candidate IDs fail. |
| `runoff_boundaries.rs` | Signed vote transfers, most-recent decisive lookback, and external tie resolutions that must contain every tied candidate exactly once and choose a winner from that tie. |
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
RUST_TEST_THREADS=2 RAYON_NUM_THREADS=2 cargo test -p velvet --locked --offline \
  --test plurality_boundaries --test results_boundaries \
  --test database_boundaries --test pipeline_boundaries
RUST_TEST_THREADS=2 RAYON_NUM_THREADS=2 cargo test -p velvet --locked --offline --test paper_pipeline
RUST_TEST_THREADS=2 RAYON_NUM_THREADS=2 cargo test -p velvet --locked --offline --lib -- boundary_tests
```

For the full native coverage report, configure Chrome and the pinned coverage
tools, fetch locked dependencies, then run from the repository root:

```sh
"$CHROME" --version
RUST_TEST_THREADS=2 RAYON_NUM_THREADS=2 DOC_RENDERER_BACKEND=inplace \
  python3 scripts/coverage/run.py velvet --baseline --offline
```

Set `CHROME` to a compatible local executable. CI uses the official Chrome for
Testing **headless shell 153.0.8010.36**, with the download and checksum pinned in
[the browser setup action](https://github.com/sequentech/step/blob/feat/meta-13302-ui-essentials-coverage/main/.github/actions/setup-test-browser/action.yml).
Both `--single-process` and `--no-zygote` remain enabled. The full Chrome binary
of the same version crashed with these flags in the isolated worker; the headless
shell passed the startup control and PDF suite. The startup control uses the
same executable resolver as the production renderer.

The full suite uses a real local browser; an absent browser is an environment
failure, not a reason to skip PDF tests. The baseline command measures progress; omitting `--baseline` enforces the local
95% line improvement target. CI instead rejects any decrease against the actual
PR base in each of lines, functions and LLVM regions. Current results and remaining work are tracked in [Meta #13302](https://github.com/sequentech/meta/issues/13302).

The report retains all measured source lines and explains files without LLVM
line regions. Existing inline tests and public fixture builders contribute to
the native aggregate. This is not production-only coverage or branch coverage.
Cloud rendering, remote storage, ACM Java signing and deployment integrations
need their own local fixture recipes before their coverage can be claimed.

When running from `packages/velvet`, Cargo defaults both the Rust test harness
and Rayon to two threads. PDF cases launch real Chrome processes; multiplying
both pools by the host CPU count can exhaust CI resources. Explicit environment
values override these defaults. Direct workspace-root Cargo invocations should set
`RUST_TEST_THREADS=2 RAYON_NUM_THREADS=2` themselves. The coverage profile and
ordinary hosted test setup now supply these settings automatically, including
`DOC_RENDERER_BACKEND=inplace` for local PDF rendering.

## Measured checkpoint and follow-up

Source `e9bcedc52b4e3d64d9b1b66ec6676a4b3195cee6`, Rust 1.96.0,
cargo-llvm-cov 0.9.1 and the pinned headless shell: **164 tests pass, none
ignored**. The isolated native profile measures **7,201/7,578 lines (95.03%)**,
**609/694 functions (87.75%)**, **8,938/9,675 LLVM regions (92.38%)**.
The preceding 157-test suite measured 7,193/7,570 lines, 607/692 functions and
8,929/9,666 regions with the same browser and toolchain. Every metric increases,
including the region percentage before rounding. Production Clippy and workspace
formatting pass with existing warnings; 75 coverage-tool tests and Ruff pass.

Both malformed external-resolution regressions failed before the small matching
fix. A valid resolution with reversed candidate order still elects its stated
winner. Other new controls cover signed transfers, chronological tie lookback,
blocked winner-output paths, nonzero invalid-vote weights, and a random snapshot
whose order differs from both sorting rules.

The remaining 377 lines/85 functions stay counted. Most are real failure paths,
not generated derives: `generate_reports/generate_reports.rs` report/result file
reads and rendering/output failures; `ballot_images/mcballot_images.rs` receipt,
CSV, manifest and output failures; `decode_ballots/*.rs` malformed numeric and
codec input; `generate_db/generate_db.rs` failed copied-database and ballot writes;
`do_tally/do_tally.rs` and `mark_winners/mark_winners.rs` missing intermediate
files. These need further valid-control failure fixtures and remain open work.
Unused fixture constructors, `HasId` adapters and diagnostic formatting are not
called merely to increase a score. No new coverage exclusions were introduced.
