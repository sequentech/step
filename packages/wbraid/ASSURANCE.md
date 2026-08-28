<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Assurance — braid v0.6

Security is a core concern of this project, so correctness/safety assurance is
tracked as a first-class category, not folded into ordinary testing. This
document records the assurance measures we have, plan, or are considering — with,
where known, an effort/value read so we can schedule them deliberately.

It complements two neighbours (and avoids duplicating them):

- `crates/braid/v0.6_spec.md` — the protocol and its security argument (esp. §5
  slots/`collides()`, §6 update-first + anti-rewrite, §10.1 version fail-fast,
  §12 forward concerns).
- `TESTING.md` — how to run the tests that exist **today** (native + wasm).

Status legend: **[IN PLACE]** works today · **[PLANNED]** decided, not yet done ·
**[CANDIDATE]** not yet assessed.

---

## 1. Model checking — `stateright` over the real implementation  [IN PLACE]

Two harnesses under `crates/braid/tests/` model-check the **real
implementation** — the real datalog, board client, wire assembly and
signatures — rather than a separate model of it:

- `model_check_symbolic.rs` — deterministic token artifacts make the
  exploration a graph; carries the fault program and the safety/liveness
  property set. Runs in the ordinary test suite.
- `model_check.rs` — real crypto end to end; checks the honest-path axioms
  the symbolic harness assumes. `#[ignore]`d (real crypto per explored
  transition); run on demand.

Everything else — design record, fault model, property catalog, how to run,
and roadmap — is in `STATERIGHT.md` (per-commit measurement log:
`STATERIGHT-log.md`). The retired vs_lift harnesses remain in-tree,
read-only, under `crates/braid/vs_lift/` for reference.

---

## 2. Property-based testing  [IN PLACE for serialization; broader candidates remain]

**In place — the serialization bijection properties.** The wire format
(`SERIALIZATION.md`) requires `ser`/`deser` to be a bijection between values
and accepted byte strings. Two proptest properties pin it:

- **P1 (round trip)**: `deser(ser(x)) == x` for arbitrary values;
- **P2 (strictness)**: `deser(b) == Ok(v)` implies `ser(v) == b` for arbitrary
  byte strings — exercised over mutations of valid encodings
  (truncate/extend/edit, the distribution that catches trailing-byte and
  length slack) and over raw random bytes.

Two suites carry them:

- `vsc/src/utils/serialization/properties.rs` — a kitchen-sink type tree
  covering every composition rule (fixed leaves, `String`, `Option`, `Vec`
  including byte vectors, arrays, nesting, `PhantomData`, group
  elements/scalars), both contexts, all-fixed and variable struct shapes.
- `braid/tests/serialization_properties.rs` — the two braid-specific
  boundaries: `ProtocolMessage` (untrusted-board bytes, pre-signature) and
  `Predicate` (anti-rewrite persistence), over valid, mutated, and random
  distributions.

**How to run.** Both suites run as part of the ordinary `cargo test` flow; to
run just them, from the workspace root:

```sh
cargo test --release -p vsc serialization::properties
cargo test --release -p braid --test serialization_properties
```

proptest generates 256 cases per property by default. Deepen a run with the
`PROPTEST_CASES` environment variable (both suites use proptest's default
configuration, so the variable is honored):

```sh
PROPTEST_CASES=65536 cargo test --release -p braid --test serialization_properties
```

(PowerShell: `$env:PROPTEST_CASES=65536` on its own line, then the `cargo
test` command.)

The properties are format-agnostic (they pin the bijection, not byte
layouts), so they survive encoding changes as the acceptance suite.

**Still candidates**: `collides()` totality/symmetry, the `AccumulatorSet`
ordering invariants, and board-client admit/anti-rewrite behaviour under
randomized message orderings.

## 3. Fuzzing  [IN PLACE for serialization; broader candidates remain]

**In place — bijection-oracle fuzz targets** (`cargo-fuzz`/libFuzzer). Every
deserializer target embeds the strictness oracle — `if let Ok(v) =
T::deser(data) { assert_eq!(v.ser(), data) }` — so coverage-guided fuzzing
hunts panics *and* canonicality violations in one pass.

- `crates/vsc/fuzz`: deserializer oracles for ElGamal and Naor-Yung
  ciphertexts, shuffle proofs, and DKG dealings (`VerifiableShare`, including
  checking-value proofs), plus two verify-boundary targets — Naor-Yung
  verify-and-strip (the PlEq verifier) and Schnorr verification — running
  accepted adversarial inputs against fixed, deterministically derived keys,
  and the pre-existing `encode_bytes`/`encode_scalar` targets.
- `crates/braid/fuzz`: oracles for `ProtocolMessage` and `Predicate`.

**How to run — vsc** (any platform; `cargo fuzz` needs the nightly toolchain,
which is this workspace's default). From `crates/vsc`:

```sh
cargo fuzz list                                        # enumerate the targets
cargo fuzz run deser_ny_ciphertext_ristretto           # fuzz until Ctrl-C
cargo fuzz run deser_ny_ciphertext_ristretto -- -max_total_time=300
```

libFuzzer options go after the `--` separator: `-max_total_time=<seconds>`
bounds a run (deeper campaigns = raise it); `-help=1` lists the rest.

**How to run — braid** (Linux only, see below). From the **workspace root**:

```sh
cargo fuzz run deser_predicate --fuzz-dir crates/braid/fuzz -- -max_total_time=300
cargo fuzz run deser_protocol_message_ristretto --fuzz-dir crates/braid/fuzz -- -max_total_time=300
```

Two platform constraints, one of which shapes the command:

- **Run from the workspace root, not `crates/braid`.**
  `crates/braid/.cargo/config.toml` applies `[unstable] build-std` and wasm
  linker flags (needed for wasm threading) to any cargo invocation started
  inside that directory, which breaks the host-side fuzz build; invoking from
  the root with `--fuzz-dir` keeps that config out of scope.
- **Windows/MSVC cannot even link these targets**: braid's wasm `cdylib`
  crate-type conflicts with libFuzzer's `/include:main` (and the
  `--no-include-main-msvc` opt-out removes the fuzz binary's own entry
  point), so both `cargo fuzz build` and `cargo fuzz run` fail at the link
  step. The targets compile (the failure is in linking) but **have not yet
  been executed — the first Linux/CI run is pending**. Until then, the braid
  property suite above exercises the same bijection oracle host-side.

Smoke baseline (2026-08-28, 40s/target, Windows host): eight vsc targets —
the six serialization-campaign targets plus `encode_bytes_ristretto` and
`encode_scalar_bytes_ristretto` — clean: ~7.9M executions, zero crashes, zero
bijection violations.

**Still candidates**: fuzzing the b4 server's own inputs, and the braid
`verify` path end to end (signature + statement reconstruction) beyond the
serialization layer.

## 4. Static analysis & linting  [CANDIDATE / hygiene]

Baseline `clippy` (ideally `-D warnings` in CI) and a warning-clean `cargo doc`.
The pre-existing `dead_code` and `broken_intra_doc_links` backlog was cleared during
the legacy retirement pass; a fresh audit may surface new items. Consider `cargo-deny` (licenses/advisories) given
the dependency surface.

## 5. Verified / audited dependencies  [CANDIDATE — not yet assessed]

Track the trust placed in cryptographic dependencies (the `cryptography`/`vsc`
primitives, `ed25519-dalek`, the group/curve backends) and prefer
audited/formally-verified implementations where practical. Record any audit
status and version pins relevant to assurance here.

---

## Design-level assurance (already in place)  [IN PLACE]

Properties the design enforces structurally, independent of any test run:

- **Total, compiler-checked `collides()`** (§5.1): every stored message maps to
  exactly one predicate, and a total collision check runs over the whole set
  before any action is trusted — equivocation ⇒ HALT, order-independent.
- **Anti-rewrite persistence** (§6.2–6.3): persisted predicate digests pin
  `H(body)`, so an untrusted b4 can withhold (availability) but never silently
  rewrite history (safety).
- **Exact-match version fail-fast** (§10.1): serialization skew across the
  b4 ↔ trustee boundary is a hard error, not silent misbehaviour.
- **Current test coverage** — see `TESTING.md` (native unit + integration incl.
  the restart/anti-rewrite SQLite test; wasm headless I/O; the interactive
  emulator for the protocol under wasm).
