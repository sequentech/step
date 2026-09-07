<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Load testing a deployed server from plain VMs

Findings and changes for running `packages/step-cli/scripts/*` against a
remote deployment from several client machines, without a dev container.
Full walkthrough: `load-testing-guide.md` (single machine, both channels)
and `distributed-load-testing.md` (several load clients), both under
`docs/docusaurus/docs/07-developers/02-cli/02-tutorials/load-testing/`.

## Findings

- No binary is git-tracked. `step-cli` is built from the full workspace
  (`sequent-core`, `windmill`, ...); `ivr-cli` lives in the private `beyond`
  submodule. The CI `step_cli_build.yml` uploads `release/seq` instead of
  `release/step-cli`, so its artifact is empty.
- Only Stage 1 (provisioning) and cleanup need `step-cli`. The online Stage 2
  needs Python + PyYAML, Node and Playwright Chromium — no Rust, no compose
  stack. Provision once, `rsync` the run dir to every client.
- The scripts already take every URL/credential from `layers.yaml` and
  accept per-client URL overrides; the missing part was toolchain bootstrap.
- Attaching only the `devcontainer` compose service is possible but pulls a
  multi-GB nix store per VM and skips the VS Code lifecycle hooks — not a
  quick setup.

## Done in this branch

- `packages/step-cli/scripts/install_load_client.sh` — idempotent apt-based
  setup; `--with-provisioner` adds Rust + `step-cli`, `--with-telephone` adds
  `ivr-cli` + local Redis.
- Playwright load test is self-contained under
  `packages/voting-portal/test/load/` (own `package.json` pinning
  `@playwright/test`, config moved next to the spec). The runner searches
  that install first and honours `PLAYWRIGHT_BIN`.
- Synchronised or ramped start, in both runners: `start_at` /
  `LOAD_TEST_START_AT` is the shared instant every client holds for after
  preflight; `start_delay` / `LOAD_TEST_START_DELAY` is a per-client offset
  on top of it, so client *i* joining at `i × step` seconds ramps the load
  up until the peak.
- `aggregate_online_load_test.py` merges per-client outputs: totals,
  wall-clock span, cast/second, p50/p95 durations, sharding-overlap warning.
- Both runners' top-level `summary.json` now carry run-wide `started_at`,
  `finished_at`, `elapsed_secs` and `cast_per_second` summed over every
  tenant (all tenants share one deployment and database).
- Client hostname and slice recorded in each run's top-level `summary.json`.

## Still open

- Ramp-up is per client only; within one client all Playwright workers (or
  `ivr-cli` calls) start at once.
- Voter sharding is manual (`voter_offset`/`max_votes` per client); an env
  override such as `LOAD_TEST_SHARD=i/N` would let one config serve all.
- Stage 2 dies if `<hasura_url>/healthz` is unreachable; on a locked-down
  deployment this should be a warning.
- Fix the CI artifact path so a prebuilt static `step-cli` is downloadable.
- Cloudflare bot protection may challenge headless Chromium; allow-list the
  clients' egress IPs before a run.
