<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
# sequent-core

Sequent shared Rust code. This code might be used in different
projects/packages, like the Ballot Verifier or the Voting Portal.

Currently this includes, among other:
 * The structures that represent an auditable ballot.
 * Methods to generate the ballot cyphertexts.
 * Methods to generate a hash from a cyphertext.

In the future this repo will also include the ballot encoder-decoder.

## Building

Within the step monorepo dev container:

```bash
cd packages/sequent-core
cargo build
```

## Generate javascript package

```bash
export RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals'
rustup run nightly-2022-04-07 wasm-pack build --out-name index --release --target web --features=wasmtest -- -Z build-std=panic_abort,std
rustup run nightly-2022-04-07 wasm-pack pack .
```

## Run rust tests

To run rust tests:

```bash
cargo test
```

## Browserstack tests

To run browserstack tests:

```bash
python3 src/wasm/test/serve.py
```

On another terminal, run this, previously configuring the env vars:


```bash
BROWSERSTACK_USERNAME=$BROWSERSTACK_USERNAME BROWSERSTACK_ACCESS_KEY=$BROWSERSTACK_ACCESS_KEY npm run local
```

## Generate JSON schema

```bash
cargo build --release
./target/release/sequent-core > ballot-schema.json
```



## Troubleshooting

### Build errors related to Permission denied errors when running cargo test or cargo clean locally.

At build errors when running `cargo test` locally it´s typically due to permission issues.
You should consider to try `cargo clean` first to start from a clean environment.
Then, if you encounter permission errors when trying to run `cargo test` or `cargo clean`, such as:

```bash
error: failed to remove file `/workspaces/step/packages/target/debug/deps/rmetaKakzvF/full.rmeta`

Caused by:
  Permission denied (os error 13)
```

This is typically due to incorrect file ownership in the `target` directory. To fix this:

1. Change the ownership of the target directory:
   ```bash
   sudo chown -R $USER:$USER /workspaces/step/packages/target
   ```

2. Run `cargo clean` to remove the build artifacts:
   ```bash
   cargo clean
   ```

3. Now you can run `cargo test` again without permission issues.
