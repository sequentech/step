---
name: fast-feedback
description: Use the repository's fast edit, preview and focused-test workflows for UI, WASM, Keycloak or Rust work. Select existing commands and fixtures, inspect affected scope, and avoid duplicate builds or unnecessary backend services.
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Fast feedback

Read repository `AGENTS.md`. The [developer guide](../../../docs/docusaurus/docs/07-developers/03-development-environment/fast-feedback.md)
contains the same workflow for developers and agents; read the section relevant
to the task. Run commands from the repository root in the configured devenv shell
or devcontainer, keeping the current checkout and its service ownership explicit.

## Select the loop

```sh
scripts/dev/step-dev --help
scripts/dev/step-dev mode status
scripts/dev/step-dev affected --worktree --json
scripts/dev/step-dev test --list
```

`affected` explains transitive consumers, selected checks and conservative
fallbacks; it does not execute tests. On a stacked branch pass `--base` with its
immediate PR base. Use `test <package|path|check>` for the current edit and
`test --affected` for fast checks across the selected changes. A no-op or skipped
check is not a passing test. `validate` includes slower checks; inspect its
`--help` for the full integration scope before starting services.

| Edit | First useful feedback |
| --- | --- |
| Portal or shared UI | Fixture-backed screen story or `packages/workbench`; consuming dev servers compile shared source directly |
| One story | `step-dev test <package> --story <story-id>`; `--watch` keeps the browser runner active |
| One unit or browser test | `step-dev test <test-file>`; the runner and scope are printed before execution |
| Rust-backed frontend logic | `step-dev wasm`, then reload/rerun the affected real WASM flow |
| Rust service | Inspect the checkout's existing watcher logs before starting another compiler; use `step-dev test <crate> <test-name>` |
| Keycloak template, message or CSS | `ui-keycloak` mode, the live theme mount and the relevant authentication page |
| Backend state | `step-dev scenario list`, then `scenario up <name>` only when a fixture-backed screen cannot answer the question |

Use `ui-only` for synthetic screens. Find stable story and workbench links in
the guide's **Screens, workbench and scenarios** section. Do not build shared
UI packages for an ordinary source edit; production journeys still need their
production outputs. `step-dev wasm --status` reports source freshness; a failed
WASM rebuild must not be reported as a successful preview of the new source.

For new regression tests follow [implement-unit-tests](../implement-unit-tests/SKILL.md)
and the package testing guide. Browser work requires checking the rendered
result and relevant interaction; bundler completion alone is insufficient.
Keep backend fixtures synthetic and resets limited to the scenario's recorded
resources. Check the Docker daemon, project and ports before a mode transition;
another checkout's services are not available for cleanup or replacement.

## Return usable evidence

Report the command, revision, selected scope and result. Include the preview URL
and scenario/story identifier when a browser surface is available. Keep process
PIDs and logs for servers started by the task; stop them when finished unless the
user asked to retain the workspace. Use `step-dev bench` for performance claims,
with cache state, load, raw samples and sample counts; do not infer latency from
one successful test run. CI uses the same selection model; compare its summary
with the local `affected --base <ref> --json` output when scope differs.
