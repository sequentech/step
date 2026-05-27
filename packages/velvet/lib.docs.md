<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

 SPDX-License-Identifier: AGPL-3.0-only
-->

`velvet` is the Rust library and binary that runs **end-to-end election tally pipelines**: decrypt and decode ballots, apply counting algorithms, generate reports (including PDFs), derive ballot images where configured, and mark contest winners.

## Module Guide

- `cli`: command-line parsing, execution state for stepping through pipeline stages, errors, and the comprehensive `test-all` harness.
- `config`: typed configuration for the tally run—election inputs, paths, and nested settings such as ballot-image and report-generation options.
- `fixtures`: reusable synthetic elections, areas, contests, candidates, and ballot styles for unit and integration tests.
- `pipes`: the tally **pipeline** itself—stage identifiers, shared errors, inputs, and individual stages.
- `utils`: cross-cutting helpers used by stages (for example JSON file parsing and decoded-vote maps).


## Generating Docs

Build the local API reference for this crate:

```bash
cargo doc -p velvet --no-deps
```
