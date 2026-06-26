<!--
SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
# Braid manual benchmark scripts

These are manual scripts for running a local 3-trustee braid demo election
and benchmarking distributed key generation / tally performance against a
running B4 bulletin board. They wrap the `demo_tool` binary (see
`packages/braid/src/bin/demo_tool.rs`) and are run by hand from this
directory — they are not wired into CI and have no automated test coverage.

## Scripts

- `build.sh` — builds braid in release mode with the `jemalloc` feature.
- `gen.sh <num_trustees> <threshold>` — generates trustee configs under `demo/`.
- `init.sh <board_count>` — initializes the protocol boards (hard-coded for 3 trustees) and clears each trustee's local message store.
- `dkg.sh` — runs distributed key generation by launching all 3 trustees (`demo/1`, `demo/2`, `demo/3`) in parallel via their generated `run.sh`.
- `run_trustee.sh` — runs a single trustee binary (`main_concurrent`) directly against a B4 board, timing it with `/usr/bin/time`.
- `ballots.sh <board_count> <ciphertexts> <num_trustees> <threshold>` — posts random ballots to the board(s).
- `tally.sh` — runs the tally/decryption phase across all 3 trustees, recording timing stats for trustee 1 to `stats.txt`.
- `go.sh` — sweeps `init.sh`/`dkg.sh`/`ballots.sh`/`tally.sh` over increasing ballot counts (10k, 20k, ... 90k) to benchmark throughput.

## Typical manual run

1. Start a B4 bulletin board server (see `packages/b4`).
2. `./build.sh`
3. `./gen.sh 3 2` (3 trustees, threshold 2)
4. `./init.sh 1`
5. `./dkg.sh`
6. `./ballots.sh 1 10000 3 2`
7. `./tally.sh`

Check `stats.txt` / `log1.txt`-`log3.txt` for timing and trustee output.
