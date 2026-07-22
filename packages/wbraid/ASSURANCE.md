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

## 1. Model checking — port the vs_lift `stateright` tests  [PLANNED, Tier 1]

The braid datalog (`crate::datalog`, §7) is a port of the vs_lift `ascent` rules;
vs_lift also carried `stateright` model-checking harnesses that were **not**
ported. Bringing them over is banked in spec §12 and carries real assurance value
for the "brain". The source is restored (read-only) under
`crates/braid/vs_lift/` for reference.

The harnesses split into two tiers with very different effort profiles.

### Tier 1 — ascent-logic model checks  [feasible, moderate]

Per-phase `stateright::Model`s for DKG / mix / decrypt (in
`vs_lift/.../ascent_logic/{dkg,mix,decrypt}.rs` `mod stateright`), a shared board
mock in `ascent_logic/mod.rs` (`HashBoard`), and a whole-protocol model in
`protocol.rs` — ~1,400 lines total. Each model: `actions()` runs the **infer**
program to get enabled actions; `next_state()` runs a test-only **execute**
program that fabricates result messages with stub hashes; `properties()` asserts
safety/liveness (e.g. "shares completed", collision ⇒ no progress); a `#[test]`
BFS-checks a small committee (e.g. `<RistrettoCtx, 2, 2, 3>`). These depend only
on `ascent_logic` + `cryptography::Context` + the `stateright` crate — **no**
actor/handler coupling — so they plug straight into `crate::datalog`.

Effort is **comparable to the datalog rule port already completed**:

- Reusable as-is: the composed **infer** program (`datalog::composed`), the rules,
  the `Action`/`Predicate` types, and the newtypes.
- To add (all native-only `#[cfg(test)]`):
  1. a `stateright` dev-dependency;
  2. a `HashBoard`-equivalent mock keyed on braid's `Predicate` (~100 lines,
     mechanical);
  3. the test-only **execute** ascent fragments (`*_execute`) — these were dropped
     in our port and must be re-added (small: a couple of rules per phase, stub
     hashes);
  4. a `composed_execute` program (analogous to the existing `composed`);
  5. the `Model`/`Property` impls per phase + the whole-protocol model
     (mechanical translation).
- Known friction (no architectural unknowns): vs_lift's positional `Message` enum
  → braid's typed named-field `Predicate` (the same adaptation already done for
  the infer input-mapping rules), the `message(…)` → `predicate(…)` relation
  rename in the execute rules, and adding the test-only `active` relation (braid's
  prelude does not carry it).
- Suggested delivery: per phase (dkg → mix → decrypt → whole-protocol), each a
  small self-contained addition under `datalog`.

### Tier 2 — integration / actor model checks  [major; a rewrite, not a port]

`vs_lift/integration_tests.rs` (~2,335 lines) + `integration_tests_basic.rs`
(~634) build a `stateright::Model` over the **full actor system**
(`trustee_administration_server::handlers`, `trustee_application::{handlers,
top_level_actor}`, `trustee_cryptography`, `trustee_messages`). v0.6 deliberately
**replaced** that architecture with `SessionTrustee` + `BoardClient` + `b4`, so
this is not portable as-is — it would mean re-expressing the integration model
over braid's runtime (`Session`/`BoardClient`), i.e. new design work. Treat as a
separate, later question if end-to-end model checking of the v0.6 runtime is ever
wanted.

**Verdict:** pursue **Tier 1** when scheduled (bounded, moderate, high
value-for-effort). **Tier 2** is a major undertaking best framed as new work.

---

## 2. Property-based testing  [CANDIDATE — not yet assessed]

Generative tests (e.g. `proptest`) over the platform-agnostic, deterministic
cores: `collides()` totality/symmetry, predicate (de)serialization round-trips,
the `AccumulatorSet` ordering invariants, and board-client admit/anti-rewrite
behaviour under randomized message sets/orderings. Overlaps with Tier 1 model
checking but is cheaper to stand up and complements it (random inputs vs.
exhaustive small-state search).

## 3. Fuzzing  [CANDIDATE — not yet assessed]

Highest-value target is the **untrusted boundary**: deserialization of
`WireMessage` / bodies and the `verify` path (`cargo-fuzz`/libFuzzer on the
`b4` → trustee inputs). b4 is untrusted (§8), so malformed/adversarial bytes must
never panic or mis-verify — a natural fuzzing surface.

## 4. Static analysis & linting  [CANDIDATE / hygiene]

Baseline `clippy` (ideally `-D warnings` in CI) and a warning-clean `cargo doc`.
Known backlog to clear first: a few pre-existing `rustdoc::broken_intra_doc_links`
warnings (in `accumulator`, `store`, the HTTP test harnesses) and a benign
`dead_code` (`util::dbg_hash`). Consider `cargo-deny` (licenses/advisories) given
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
