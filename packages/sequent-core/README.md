<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
# sequent-core

Sequent shared Rust code. This code might be used in different
projects/packages, like the Ballot Verifier or the Voting Portal.

This includes auditable ballot types, ballot encoding and decoding, wbraid
Naor–Yung encryption and audit openings, ballot trackers, signatures and
receipts. Native trustee DKG, mixing and decryption are handled by
`protocol-board` and the supplied `packages/wbraid` workspace.

## Building

Within the step monorepo dev container:

```bash
devenv shell -- bash -c 'cd packages && CARGO_TARGET_DIR="$PWD/rust-local-target" cargo build -p sequent-core --features default_features'
```

## Generate javascript package

From the repository root, see the fast feedback loops developer guide:

```bash
scripts/dev/step-dev wasm                  # development package for dev servers
nix develop ./packages/sequent-core --command \
  scripts/dev/step-dev wasm --release-package  # committed tgz files
scripts/dev/step-dev wasm --check-package  # committed tgz matches the sources
```

## Run rust tests

To run rust tests:

```bash
devenv shell -- bash -c 'cd packages && CARGO_TARGET_DIR="$PWD/rust-local-target" cargo test -p sequent-core --features default_features --lib'
```

## Browserstack tests

The smoke page encrypts a fresh canonical wbraid sample ballot, checks its tracker,
and verifies its audit opening with the installed `sequent-core` WASM package.
From the repository root, first install the Yarn workspace dependencies and check
the committed WASM package, then serve the page:

```bash
python3 packages/sequent-core/src/wasm/test/serve.py
```

Open `http://localhost:8080/sequent-core/src/wasm/test/test.html`. A successful
run displays `verified: <ballot tracker>`. To run the same page on BrowserStack,
install the dependencies in `packages/sequent-core/browserstack`, set your
BrowserStack credentials, and run:


```bash
cd packages/sequent-core/browserstack
npm run local
```

## Generate JSON schema

```bash
devenv shell -- bash -c 'cd packages && CARGO_TARGET_DIR="$PWD/rust-local-target" cargo build -p sequent-core --release --features default_features'
packages/rust-local-target/release/sequent-core > ballot-schema.json
```



## Troubleshooting

Use this checkout's `packages/rust-local-target` for Cargo. The Docker-owned
`packages/target` directory belongs to service builds; do not change its ownership
or clean another checkout's build directory. See the
[build and test guide](../../docs/docusaurus/docs/07-developers/03-development-environment/build-and-test.md)
and [core testing guide](../../docs/docusaurus/docs/07-developers/03-development-environment/testing/sequent-core.md).
