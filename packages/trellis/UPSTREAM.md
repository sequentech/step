<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Trellis upstream

Imported from https://github.com/ruescasd/mrkl/tree/trellis at commit
`57ddd6d171ae6fc9f1545a1302f2d1e82f2defb7`. Original source, examples and
tools are retained. Upstream does not include a license file or Cargo license
field at this revision; this import does not assign it a new license.

Step uses the tree/proof implementation with `journal`, a transactional
PostgreSQL adapter. The original source-table polling server and tools are
available behind `upstream-service`; they are not started by Step.
The package uses Step's workspace lockfile.

Local correctness fixes require a trusted tree size when verifying consistency
proofs. The upstream HTTP client retains complete root/size checkpoints, and the
monitor validates their ordering, bounds growth-race retries, and keeps its last
verified checkpoint after any failure. Run its adversarial HTTP regressions with
`cargo test -p trellis --features upstream-service --example monitor`.
