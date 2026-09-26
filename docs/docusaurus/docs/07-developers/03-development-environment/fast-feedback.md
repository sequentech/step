---
id: fast-feedback
title: Fast feedback loops
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The development loop is edit, see the result, run the relevant test. Commands in
this guide run from the repository root inside the devcontainer unless stated;
`scripts/dev/step-dev <command> --help` describes each one.

## Devcontainer modes

## Shared UI hot reload

## Screens, workbench and scenarios

## Incremental WASM

## Focused tests

## Benchmarks

`scripts/dev/step-dev bench` times these loops for any checkout given with
`--checkout`, so two trees can be compared on the same machine. Each series is
one JSON file under `<output-dir>/<scenario>/` (default
`$STEP_BENCH_OUTPUT_DIR`, else `~/.cache/step-bench/results`) with the commit,
tool versions, load average around every sample, services, cache state,
commands and raw samples. Warm series run untimed warm-up iterations, then ten
samples by default.

```sh
B="scripts/dev/step-dev bench"
$B ui-update --label before --checkout . --edit shared-header \
  --target voting --target admin --target verifier --target results \
  --rebuild-cmd 'yarn --cwd packages build:ui-essentials'
$B ui-update --label before --checkout . --edit voting-screen --target voting
$B ui-update --label before --checkout . --edit shared-header --target storybook
$B test --label before --checkout . --suite cargo-harvest
$B rust --label before --checkout . --edit windmill-service --build windmill --build harvest
$B wasm --label before --checkout . --edit sequent-core-wasm
$B summarize ~/.cache/step-bench/results --phases
```

Edits insert a unique marker line and restore the file afterwards. `ui-update`
starts its own dev servers from `--port-base` and times until headless Chromium,
authenticated through the `ui-test-kit` mocks, shows the marker. `test` suites
cover Jest, a Storybook story and Cargo. `wasm` restarts the dev server after
`--build-cmd` and `--install-cmd` unless `--server-restart never`; `--no-change`
repeats the workflow without an edit.

`workspace` and `ci` run on the host, with the Dev Containers CLI and `gh`.
`workspace` never uses the default Docker daemon: a cold sample creates a fresh
Docker-in-Docker daemon and checkout copy under `--sandbox-root`, and `--keep`
leaves them for warm samples, which stop the stack and time the next start
until its readiness probes pass. `clean` removes a kept daemon.

```sh
$B workspace --label before --checkout . --cache cold --keep --sandbox-root ~/bench
$B workspace --label before --checkout ~/bench/envs/step-bench-dind-1/step \
  --cache warm --docker-host unix://$HOME/bench/dind/step-bench-dind-1/run/docker.sock
$B ci --label before --pr "$PR"
```

## Incremental CI
