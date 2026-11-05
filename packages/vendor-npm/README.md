<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

`vitrine-verify.tgz` and `vitrine-ui.tgz` are Apache-2.0 packages packed from the immutable Vitrine commit in `VITRINE_COMMIT`. `SHA256SUMS` records the exact artifacts installed by the three frontend apps.

Refresh with `scripts/vendor-vitrine.sh /path/to/vitrine <commit-or-ref>`, then run `yarn install --ignore-scripts` in `packages/`. The script archives the selected commit’s packages, test vectors and normative specs into a temporary directory, installs its locked dependencies, builds and tests the packages, and packs them. Including the specs runs the UI’s evidence-wording coverage checks instead of skipping them. It leaves the source checkout untouched. Commit both tarballs, the commit pin, checksums and the Yarn lockfile together. `sha256sum -c SHA256SUMS` verifies the checked-in artifacts.
