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

## 1. Model checking — `stateright` over the real implementation  [IN PLACE on `exp/braid-stateright/main`]

Explicit-state model checking exists as **two harnesses that drive the real
implementation** — the real `datalog::composed::run`, the real `BoardClient`
(committed set §6.2/§6.3, outgoing mailbox §6.4), real wire assembly and
signatures — rather than a port of the vs_lift harnesses. Design record,
measurements and roadmap: `STATERIGHT.md`. Currently on the experimental
branch; not yet mainlined.

- `crates/braid/tests/model_check.rs` — real crypto end to end
  (`Trustee::step`). Explores interleavings as a tree (nondeterministic crypto
  never dedupes); checks the honest-path axioms: no trustee halts, and some
  interleaving publishes exactly the encrypted inputs. `#[ignore]`d — run
  explicitly (real crypto per explored transition).
- `crates/braid/tests/model_check_symbolic.rs` — the real datalog, wire
  assembly and signatures over **deterministic token artifacts**; canonical
  order-free state identity makes the exploration a graph, so completion is
  checked as a strong `eventually` over ALL paths. The fault program lives
  here: faults are budgeted actions with provenance in the state, and the
  first result stands — §6.4's compute-once/send-until-acked verified over
  every ≤ 2-dropped-commit pattern and interleaving at n=2 (no halts, every
  path completes). Runs in the ordinary test suite (~1s, several committee
  configurations).

The port plan this section previously carried is **retired**, deliberately:

- *Tier 1* (per-phase ascent-logic models with re-added stub-hash `execute`
  fragments and a `HashBoard` mock) would have created a second, test-only
  rendering of the protocol's action layer next to the real one — a drift
  surface. The harnesses above check the real thing instead, which is strictly
  stronger and turned out no more expensive.
- *Tier 2* (the vs_lift integration/actor model, ~2,970 lines) remains
  unportable as-is (v0.6 replaced that architecture), but it was **mined**: its
  fault/property catalog — budgeted fault actions, fault provenance in state,
  fault-conditioned safety/liveness, non-vacuity guards, no-exemption
  agreement properties — is adopted as the template for the fault program (see
  `STATERIGHT.md`).

The vs_lift source stays restored (read-only) under `crates/braid/vs_lift/`
for reference.

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
