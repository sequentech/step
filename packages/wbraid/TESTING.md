<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Testing braid v0.6

The v0.6 stack (the `braid` trustee + the `b4` bulletin board) runs in two
environments — **native** and **wasm** — and is tested separately in each. See
`crates/braid/v0.6_spec.md` for the protocol itself.

The two environments also carry the two persistence backends
(`SqlitePersistence` native, `IndexedDbPersistence` wasm); each is covered within
its section below.

## Native

The default build (`default = ["native"]`), driven with `cargo`.

### Unit + integration tests

```sh
# From the repo root (wbraid/). --release because the crypto is slow in debug.
cargo test -p braid --release
```

- **Crate unit tests** — the datalog engine and accumulator, and the board client:
  the anti-rewrite boundary check, the DKG/tally board union, and the
  restart/anti-rewrite persistence test
  (`persisted_predicate_blocks_rewrite_across_restart`), which is what exercises
  `SqlitePersistence` (the native persistence backend).
- **`tests/test_protocol.rs`** (the end-to-end harnesses) — these use
  `NoOpPersistence` (a clean run does not exercise persistence/restart):
  - `test_protocol_memory` — DKG → mix → threshold-decrypt over an in-memory
    mock b4.
  - `test_protocol_memory_union` — one DKG, one tally over a child board unioned
    with it (§8.2).
  - `test_protocol_memory_union_batches` — one DKG reused by several tallies, each
    on its own child board (the union-as-batch mechanism).

### Live-b4 tests (opt-in)

Two harnesses talk to a real `b4` over HTTP and are `#[ignore]`d so the default
run stays hermetic. Both need `b4` **and** S3/LocalStack: b4 stores every message
body in S3 (`MAX_INLINE_MESSAGE_SIZE = 0`), so any live-b4 run touches S3
regardless of the test. They differ in the *client* board setup:

- `test_protocol_http` — single board, `NoOpPersistence` (no client-side
  persistence).
- `test_protocol_http_union` — DKG/tally board union with client-side
  `SqlitePersistence`.

```sh
# Terminal 1:  .\localstack.ps1     (S3 via LocalStack)
# Terminal 2:  .\b4.ps1             (b4 server on :3000)
# Terminal 3:
cargo test -p braid --release -- --ignored
```

### Prerequisites

- A stable Rust toolchain (for the default build).
- For the live-b4 tests: **Docker** + the **AWS CLI** — `localstack.ps1` starts
  LocalStack, creates the `wbraid-messages` bucket, and applies `s3-cors.json` —
  and the **`b4`** server (`b4.ps1` sets the S3 endpoint/credentials and points
  `DATABASE_URL` at a repo-root `b4.db`).

## Wasm

The trustee also compiles to `wasm32-unknown-unknown` and runs in the browser.
Two things are tested: the IndexedDB persistence backend (headless), and the full
protocol running under wasm (interactively).

The protocol itself **cannot** be tested headless. The crypto is rayon-parallel,
so a wasm build that runs it needs the `wasm-bindgen-rayon` thread pool → the
atomics / shared-memory build → an async executor that needs `Atomics.waitAsync`
on `SharedArrayBuffer` (cross-origin isolation via COOP/COEP). The `wasm-bindgen`
headless test runner serves plain HTTP without that isolation, so the atomics
build's async tests can't run there (they fail with `memory access out of
bounds`). So the headless test is restricted to threadless I/O, and the
protocol-under-wasm is validated interactively in a real browser (which can
provide COOP/COEP) — backed by the native protocol tests above.

### Headless persistence test (I/O only)

```sh
# From the repo root (wbraid/), NOT crates/braid.
.\test-wasm.ps1
```

Runs a `wasm-bindgen-test` (`tests/wasm_indexeddb.rs`) exercising the
`IndexedDbPersistence` round-trip + idempotency in headless Chrome.

- Uses the **`wasm-core`** feature (no `wasm-bindgen-rayon`, hence no atomics), so
  it runs in plain headless Chrome with no SharedArrayBuffer / COOP setup.
- **Run from the repo root**, so the atomics `.cargo/config.toml` under
  `crates/braid` is not applied. Also ensure `RUSTFLAGS` is empty — a leftover
  `-C target-feature=+atomics…` from a prior production build silently produces a
  shared-memory binary that then fails here (`$env:RUSTFLAGS=""`).
- Set `NO_HEADLESS=1` to watch the browser.

### Interactive emulator (full protocol under wasm)

`emulator.html` runs browser-hosted trustees against a live `b4`, driving the full
protocol a round at a time — the production-shaped setting and the primary
validation that the protocol runs correctly under wasm.

```sh
# Terminal 1:
.\localstack.ps1        # docker LocalStack + creates the S3 bucket & CORS
# Terminal 2:
.\b4.ps1                # b4 server on :3000 (SQLite + S3)
# Terminal 3:
.\serve.ps1             # clears RUSTFLAGS, builds the wasm client (build-wasm.ps1,
                        # nightly + atomics + wasm-bindgen-rayon), then serves on
                        # :8080 with COOP/COEP (server.py)
```

Then open <http://127.0.0.1:8080/emulator.html> and:

1. **Create setup** — generates a committee, creates the DKG board on b4, posts
   the configuration.
2. **Step to fixpoint** — runs the DKG.
3. **New tally** → **Step to fixpoint** → **Verify** — mixes and threshold-decrypts
   a fresh ciphertext set on a child board unioned with the DKG; Verify confirms
   the plaintexts match the encrypted inputs.
4. **New tally** again — reuses the same DKG (the batch mechanism, §8.2).
5. **Export / Import** — export the setup to a paste string, refresh the page,
   paste it back, and Import to reconnect the same boards + IndexedDB stores
   (the bridge for testing persistence across a refresh).

Validates the full DKG → mix → threshold-decrypt under wasm; live b4 + S3;
per-trustee IndexedDB persistence; the DKG/tally board union and multiple tallies
over one DKG; and the export/import Setup bridge. Manual by design (see the note
above).

### Prerequisites

- The `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`).
- **`wasm-bindgen-test-runner`** (ships with `wasm-bindgen-cli`, version-matched to
  the pinned `wasm-bindgen`) and **Chrome + `chromedriver`** on `PATH` — for the
  headless test.
- A **nightly** toolchain — for the production wasm build (`build-wasm.ps1` sets a
  nightly override under `crates/braid`, whose `.cargo/config.toml` forces the
  atomics target-features), plus **Python** for `server.py` (the COOP/COEP dev
  server).
- For the emulator: **Docker/LocalStack** + **`b4`**, as in the Native live-b4
  prerequisites.
- **`RUSTFLAGS` caveat** — clear any inherited `RUSTFLAGS` before the headless test
  (and run it from the repo root); `serve.ps1` clears it itself before the
  production build.
