<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Agent guidance

Sequent Voting Platform: an end-to-end verifiable online voting system.
`packages/` is both a Cargo and a Yarn workspace; Rust also compiles to WASM
for the portals. Developer docs: `docs/docusaurus/docs/07-developers/`, starting
with the [contributing guide](docs/docusaurus/docs/07-developers/00-contributing.md).

## Rules

- Every file has REUSE (SPDX) headers; `reuse lint` passes.
- Branches: `feat/meta-<issue>-<description>/<target>`, or
  `fix/meta-<issue>/release/X.Y` for release fixes. PR bodies start with
  `Parent issue: https://github.com/sequentech/meta/issues/<n>`.
- Never force-push or rebase a shared branch; merge instead.
- TDD: a failing test first, then the code.
- Policies are enums, not booleans.
- No client-specific features; design for every client.
- Data changes stay backwards compatible (optional or defaulted fields).
- Cargo builds into this checkout's `packages/rust-local-target`, never
  `packages/target`; leave other checkouts' services alone.

## Skills

- [fast-feedback](.agents/skills/fast-feedback/SKILL.md): edit, preview and focused-test loop.
- [build-and-test](.agents/skills/build-and-test/SKILL.md): per-stack build, test, lint, WASM, GraphQL, Hasura.
- [code-standards](.agents/skills/code-standards/SKILL.md): product and code rules, feature checklist.
- [implement-unit-tests](.agents/skills/implement-unit-tests/SKILL.md): regression tests and coverage.
- [github-workflow](.agents/skills/github-workflow/SKILL.md): branches, stacks, PRs, reviews, issues.
