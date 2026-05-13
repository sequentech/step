<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# workbench

A self-contained sandbox for exercising production components of the Sequent
Voting Platform in isolation, without external services or remote calls.

This is **not** specific to the voting portal. It is a general workbench:
production components are wrapped in thin runnable harnesses inside this
folder, sharing a common mock environment (state, scenarios, fixtures). The
initial deliverable is the voter flow plus plaintext tally counting, but the
architecture is intended to accommodate additional components over time
(encoder/decoder inspection, ballot-verifier integration, etc.).

## Subfolders

- `velvet-core/` — pure-computation subset of the `velvet` tally crate,
  extracted to compile to `wasm32-unknown-unknown` so the tally algorithms
  can run in-browser. Eventually upstreamable as a general improvement to
  `velvet`.

More to come.

## Embedding strategy

Some workbench dependencies are **shared source** (e.g. `velvet-core` is a
real crate that production also consumes) while others are **lifted** —
re-hosted production source files behind the workbench's Vite build. Lifts
require ongoing maintenance to stay faithful when the upstream code evolves.

See [LIFTING.md](LIFTING.md) for the full, replayable procedure used to
embed the voting-portal: the inventory of adaptations, the canary symptoms
that signal drift, the refresh procedure, and the rules about what may and
may not be modified.

## Status

Phase 0: extracting `velvet-core` from `velvet`.
