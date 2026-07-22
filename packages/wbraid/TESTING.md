<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Testing braid v0.6

This describes how the v0.6 stack (the `braid` trustee + the `b4` bulletin board)
is tested. See `crates/braid/v0.6_spec.md` for the protocol itself.

## Philosophy: three layers

Coverage is split across three complementary layers rather than a single
"run-all" suite, because the trustee runs in two very different environments
(native and wasm) with two different persistence backends, and because one
otherwise-obvious test is infeasible:

| Layer | Runs | Persistence | Covers |
|---|---|---|---|
| 1. Native | `cargo test` | in-memory / SQLite | protocol logic, board client, datalog, crypto |
| 2. Wasm headless | `test-wasm.ps1` | IndexedDB | wasm build + serialization + async I/O round-trip |
| 3. Interactive browser | `emulator.html` | IndexedDB | the full protocol running **under wasm** against a live b4 |

**Why the protocol isn't tested headless under wasm.** The crypto is
rayon-parallel, so a wasm build that runs it needs the `wasm-bindgen-rayon`
thread pool, which needs the atomics / shared-memory build, whose async executor
needs `Atomics.waitAsync` on `SharedArrayBuffer` (cross-origin isolation via
COOP/COEP). The `wasm-bindgen` headless test runner serves plain HTTP without
that isolation, so an atomics build's async tests cannot run there (they fail
with `memory access out of bounds` / "not a shared typed array"). Layer 2 is
therefore restricted to threadless I/O; the protocol running correctly under wasm
is validated by Layer 3 (a real browser, which *can* provide COOP/COEP) plus the
native protocol tests of Layer 1.

**Two persistence backends.** `SqlitePersistence` (native) is exercised by
Layer 1; `IndexedDbPersistence` (wasm) by Layers 2 and 3. The anti-rewrite
`collides()` logic they guard is platform-agnostic and covered natively, so the
wasm layers only need to exercise the DB I/O and the end-to-end behaviour.

---

## Layer 1 — Native (fast, default)

The default build (`default = ["native"]`), run with `cargo`.

```sh
# From the repo root (wbraid/). --release because the crypto is slow in debug.
cargo test -p braid --release
```

This runs:

- **Crate unit tests** — the datalog engine and accumulator, and the board client:
  the anti-rewrite boundary check, the DKG/tally board union, and the
  restart/anti-rewrite persistence test (`persisted_predicate_blocks_rewrite_across_restart`),
  which is what exercises `SqlitePersistence` (the native persistence backend).
- **`tests/test_protocol.rs`** (the end-to-end harnesses) — these use
  `NoOpPersistence` (a clean run does not exercise persistence/restart):
  - `test_protocol_memory` — DKG → mix → threshold-decrypt over an in-memory
    mock b4.
  - `test_protocol_memory_union` — one DKG, one tally over a child board unioned
    with it (§8.2).
  - `test_protocol_memory_union_batches` — one DKG reused by several tallies, each
    on its own child board (the union-as-batch mechanism).

### Live-b4 native tests (opt-in)

Two harnesses talk to a real `b4` over HTTP and are `#[ignore]`d so the default
run stays hermetic. Both need `b4` **and** S3/LocalStack: b4 stores every message
body in S3 (`MAX_INLINE_MESSAGE_SIZE = 0`), so any live-b4 run touches S3
regardless of the test. They differ in the *client* board setup:

- `test_protocol_http` — single board, `NoOpPersistence` (no client-side
  persistence).
- `test_protocol_http_union` — DKG/tally board union with client-side
  `SqlitePersistence`.

Run them with `b4` + LocalStack up:

```sh
# Terminal 1:  .\localstack.ps1     (S3 via LocalStack)
# Terminal 2:  .\b4.ps1             (b4 server on :3000)
# Terminal 3:
cargo test -p braid --release -- --ignored
```

---

## Layer 2 — Wasm headless (I/O only)

A headless-Chrome `wasm-bindgen-test` that exercises the IndexedDB persistence
backend (round-trip + idempotency).

```sh
# From the repo root (wbraid/), NOT crates/braid.
.\test-wasm.ps1
```

Key points:

- Uses the **`wasm-core`** feature (no `wasm-bindgen-rayon`, hence no atomics),
  so it runs in plain headless Chrome without any SharedArrayBuffer / COOP setup.
- **Run from the repo root**, so the atomics `.cargo/config.toml` under
  `crates/braid` is not applied. Also ensure `RUSTFLAGS` is empty in your shell —
  a leftover `-C target-feature=+atomics…` from a prior production build silently
  produces a shared-memory binary that then fails here (`$env:RUSTFLAGS=""`).
- Set `NO_HEADLESS=1` to watch the browser.

Prerequisites: the `wasm32-unknown-unknown` target
(`rustup target add wasm32-unknown-unknown`), `wasm-bindgen-test-runner` (ships
with `wasm-bindgen-cli`, version-matched to the pinned `wasm-bindgen`), and a
`chromedriver` matching your Chrome, both on `PATH`.

---

## Layer 3 — Interactive browser emulator (full protocol under wasm)

`emulator.html` runs browser-hosted trustees against a live `b4`, driving the
full protocol a round at a time. This is the production-shaped setting and the
primary validation that the protocol runs correctly under wasm.

```sh
# Terminal 1:
.\localstack.ps1        # docker LocalStack + creates the S3 bucket & CORS
# Terminal 2:
.\b4.ps1                # b4 server on :3000 (SQLite + S3)
# Terminal 3:
.\serve.ps1             # clears RUSTFLAGS, builds the wasm client (build-wasm.ps1,
                        # nightly + atomics + wasm-bindgen-rayon), then serves
                        # on :8080 with COOP/COEP (server.py)
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

What it validates: the full DKG → mix → threshold-decrypt under wasm; live b4 +
S3; per-trustee IndexedDB persistence; the DKG/tally board union and multiple
tallies over one DKG; and the export/import Setup bridge. This layer is **manual
by design** — see the philosophy note above for why it cannot be automated
headless.

---

## Prerequisites & environment

- **Docker** — for LocalStack (S3). `localstack.ps1` starts it, creates the
  `wbraid-messages` bucket, and applies `s3-cors.json`.
- **AWS CLI** — used by `localstack.ps1` to create the bucket and set CORS.
- **`b4` environment** — `b4.ps1` sets the LocalStack S3 endpoint/credentials and
  points `DATABASE_URL` at a repo-root `b4.db`.
- **Nightly toolchain + `wasm32-unknown-unknown` target** — for the production
  wasm build (`build-wasm.ps1` sets a nightly override under `crates/braid`).
- **`wasm-bindgen-cli`** (version-matched to the pinned `wasm-bindgen`) and
  **Chrome + `chromedriver`** — for Layers 2 and 3.
- **`RUSTFLAGS` / atomics caveat** — `crates/braid/.cargo/config.toml` forces
  the atomics target-features for the production wasm build. Run wasm **tests**
  (Layer 2) from the repo root so that config is not applied, and clear any
  inherited `RUSTFLAGS` before running them. `serve.ps1` clears `RUSTFLAGS`
  itself before the production build.
