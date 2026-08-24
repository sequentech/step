<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Stateright model checking — braid v0.6

Living design & progress document for the model-checking effort on
`exp/braid-stateright/main`. It records the plan of record, what has been built
and measured, and what comes next. Complements:

- `ASSURANCE.md` — the assurance overview. (Its §1 predates this work and
  frames model checking as a *port* of the vs_lift harnesses; what was built
  instead is described here — the vs_lift code was mined for patterns, not
  ported.)
- `crates/braid/v0.6_spec.md` — the protocol and its security argument.
- The harness module docs — `crates/braid/tests/model_check.rs` and
  `crates/braid/tests/model_check_symbolic.rs` carry the detailed design notes
  in place; this file is the map, they are the territory.

## Premises

1. **Check the real implementation.** Transitions drive the real
   `datalog::composed::run`, the real `BoardClient` (committed set §6.2/§6.3,
   outgoing mailbox §6.4), real wire assembly and signatures. There is no
   second rendering of the protocol rules to drift from the first — the
   founding lesson of the abandoned datalog-translation investigation.
2. **Nondeterministic branching among deterministic edges.** All uncertainty
   (scheduling, faults) lives in the *action set*, explored exhaustively;
   `next_state(state, action)` is a function. Faults are action parameters,
   never randomness inside a transition.
3. **Properties are claims over (fault model, exploration bound).** Every
   modeling choice (real vs token crypto, what is in the state) is judged per
   (property, fault) pair: does the property's truth depend on anything the
   abstraction fails to preserve, under these faults?

## The two harnesses

| | `model_check.rs` (v1, crypto) | `model_check_symbolic.rs` (v2, symbolic) |
|---|---|---|
| Datalog, predicates, wire assembly, signatures, board client, persistence | real | real |
| Artifact bodies | real crypto (`Trustee::step`) | deterministic tokens (`execute_symbolic`) |
| Exploration | tree (ThreadRng ⇒ no folding) | graph/DAG (canonical identity + folding) |
| Terminal property | `sometimes` completes + plaintexts == inputs | strong `eventually` completes |
| Role | honest-path axioms: honest artifacts verify (Fiat–Shamir domains agree), decryption recovers the inputs | protocol logic: interleavings, halting, collisions, lineage; the fault program |
| Cost | `#[ignore]`d (real crypto per edge) | in the ordinary suite (~1s for all configs) |

The split is deliberate: v2 *assumes* the symbolic axioms (honest artifacts
verify; forged ones don't) and checks everything the protocol builds on top;
v1 checks that the axioms hold of the real crypto on the honest path. Breaking
the axioms themselves is cryptanalysis, out of scope for any model checker.
(vs_lift's own mixnet harness was a hybrid — real crypto under hash-level
messages — paying v1's price without collecting v2's reward; that comparison
confirmed the split.)

Shared design elements (details in the module docs):

- **State = durable state only**: board bytes, staging area, committed sets,
  own-post records, fault provenance. Every transition rehydrates a real
  `BoardClient` from it — each explored edge doubles as a restart, the path
  §6.3/§6.4 make claims about. Keys live in the model (fixed per run).
- **Lookahead**: `actions()` computes each candidate turn's successor once,
  memoized; only productive turns are offered, and `next_state` serves the
  stored edge (deterministic replay of counterexamples/discoveries).
- **Two-clause no-change guard**: an idle cycle is discarded *before* durable
  updates (observation-timing compression — sound while the board is honest;
  must be revisited when rewrite faults land), and a productive cycle is
  pruned only if `next == last` (general self-loop kill).
- **Canonical state identity** (v2): board and records sorted (Configuration
  pinned first), duplicate board rows deduped (§8.5 Note 2). Sound because the
  protocol layer is order-insensitive: datalog consumes predicate sets, stores
  are content-addressed.
- **Transport model** (v2): b4's real shape — a staging area (S3-analogue, in
  the state) distinct from committed board rows, and per-trustee views (the
  board minus `withheld[i]`). The seam for stage/commit faults and split views.
- **Token discipline** (v2): a token is a deterministic function of the
  action's hash-bound inputs, including the producer index exactly when the
  real artifact would differ per trustee (shares, mixes, partial decryptions)
  and omitting it where trustees must agree by hash (public key, plaintexts).
  A validity slot is planned for forged-proof faults (Dolev-Yao style:
  validity is a property of the term; the modelled verifier reads it).

## Fault model

Two tiers over one mechanism (fault = budgeted `Turn` variant; adopted from
vs_lift's integration model, whose fault/property catalog was mined in place
of a port):

- **Accidental faults** — budgeted (`FaultBudgets` in the model,
  spent-counters in `FaultRecord` in the state). Exploring all resolutions of
  bounded nondeterminism yields **k-fault-tolerance claims**: "under any
  pattern of ≤ k such faults, X." Unconditional safety and completion are the
  target properties.
- **Adversarial behavior** (untrusted b4, dishonest manager, < threshold
  dishonest trustees) — unbudgeted. Only safety and possibility claims are on
  offer (an adversary can always deny liveness); liveness gets *conditioned*
  forms ("completes ∨ adversarial fault affected it").
- **Non-vacuity guards**: per enabled fault class, `sometimes` "the fault
  fired" (and "recovery happened despite it") — a fault model that never fires
  must not silently pass its properties.
- **No-exemption properties**: the crown-jewel claims (chain/lineage
  uniqueness) hold *unconditionally over the whole fault space* — faults may
  stall progress or be detected, but claimed success with silent inconsistency
  is always a violation.

Property scope (the checks × faults table; ✔ = token-sufficient):

| Check | Faults | v2 sufficient? |
|---|---|---|
| No honest trustee halts | benign (budgeted) | ✔ |
| Anti-self-equivocation (§6.4): one artifact per slot per honest sender | benign | ✔ |
| Anti-rewrite (§6.3): never act past a dropped/replaced committed message | adversarial board | ✔ |
| Halt-on-equivocation (dishonest trustee / manager) | adversarial | ✔ |
| Verification-failure response (forged proof ⇒ no signature) | adversarial (validity tags) | ✔ |
| Single-lineage decryption: all honest partial decryptions descend from ONE ballots hash | adversarial (paradigm: manager ballots-equivocation → differencing attack) | ✔ |
| Completion / k-fault cone (≤ k benign faults never make success unreachable) | benign | ✔ |
| Honest artifacts verify; plaintexts == inputs | — (honest path) | ✖ → v1 |
| Confidentiality as secrecy | — | ✖ → neither (knowledge modeling, Tamarin-class); only its behavioral **shadows** (the uniqueness rows above) are checkable here |

## Done (commits on `exp/braid-stateright/main`)

| Commit | What | Measurement |
|---|---|---|
| `9d1a62dba9` | Harness v1: real stack over `MemoryTransport`, n=2/t=2/W=2/2 ballots | 153 states, 0.46s, both properties |
| `9b5be76846` | `MemoryPersistence` moved into test code | — |
| `bbf98a71c4` | Lookahead + memoized deterministic `next_state`; two-clause guard | same graph (153) |
| `c338a5e733` | Harness v2: symbolic tokens, strong `eventually` | 153 = v1's tree exactly (structural cross-validation; zero folding) |
| `95b8a4c5c1` | Canonical order-free state identity; completion = mixing quorum (property fix found by n=3/t=2 run, confirmed in `decrypt.rs`) | n=2 t=2: **35**; n=3 t=3: 262; n=3 t=2: 253 |
| `62f628e1a1`, `acac82a755` | `Action::ComputeBallots` removed (Q13 resolved: our harnesses model the manager as an actor) | — |
| `a57c222d84` | Transport model: stage/commit split + per-trustee views | counts unchanged (behavior-preserving fault-free) |
| `2048b8df27` | Fault scaffolding + `DropCommit` (first benign fault) | n=2 t=2 drops≤2: 187 states; **no halts, every path completes, guards fire** — §6.4 send-until-acked verified over every ≤2-drop pattern |
| `fa22aabf29` | Merge of `feat/braid-0.6.2/main` (vsc: deser-cursor fix, fold strategy, benchmark, lint) — made on top of the stateright work and pulled in | all suites green post-merge; symbolic state counts byte-identical (the deser fix perturbed nothing) |
| `9c6a687c95` | `CrashBeforeRecord` (second benign fault): real `post` aborted at the record write via a failing-persistence delegate; death-not-halt semantics; budget spent only when the crash fires | crashes≤2: 187 states (isomorphic to drops≤2 — the two seam-sides mirror); mixed drops≤1+crashes≤1: 303 states; all properties + guards |
| `c55d0d3b53` | Fetch-failure stutter lemma pinned (benign tier complete): read failures have zero durable footprint ⇒ schedule-equivalent, no fault class needed; module docs gain "deliberately NOT fault classes" | lemma test green against the real `BoardClient::update` |
| `fd6c60dd2c` | `EquivocateBallots` (first adversarial fault, the differencing-attack row) + per-trustee halting (a halted trustee loses its turns; others run on — the global freeze would have hidden the dangerous interleavings) + the no-exemption lineage property on every state of every config | equivocations≤1: 135 states; conditioned safety + required-response liveness + 3 guards all verified; benign counts unchanged. Caveat: with a consistent board the second lineage can never begin (seeing ballots#2 implies seeing ballots#1 ⇒ halt); the sharp variant is equivocation + split views |

## Next steps (plan of record)

Agreed order for the fault program:

3. **Benign tier — COMPLETE.** Write path: `DropCommit` (`2048b8df27`, after
   the §6.4 commit point — covers b4 loss and pre-b4 failures alike, since the
   resulting state is one) + `CrashBeforeRecord` (`9c6a687c95`, before it);
   stage failures are behaviorally subsumed by the latter. Read path: benign
   fetch failures are **stutters, not a fault class** — `update()` fetches
   everything before admitting anything, so a read failure aborts with zero
   durable footprint, and schedule exploration already covers any finite
   number of them; the atomic-abort assumption is pinned by the
   `fetch_failure_has_no_durable_footprint` test (`c55d0d3b53`).
   Duplicate delivery is absorbed (board rows dedup, store is
   predicate-keyed). Withholding has **no benign version** (b4 is one
   consistent store: a fetch returns the board or an error) — it is
   adversarial by nature and moves to step 4.
4. **Adversarial tier**: ~~manager ballots-equivocation~~ (DONE, `fd6c60dd2c`
   — verified under a consistent board, where the collision halts every
   trustee before a second lineage can begin). Remaining: **split views /
   withheld messages** (adversarial b4) — including the sharp differencing
   variant, equivocation + withholding, where b4 hides one ballots from some
   trustees; and **forged proofs** via token validity tags (SignMix reads the
   tag instead of signing unconditionally). Properties: the no-exemption
   lineage row is already in place on every config; conditioned variants per
   class. The withholding work carries the two parked rulings: the
   **observation-timing compression** (`admit()` pins every fetched
   predicate, so real trustees pin on every fetch; the compression discards
   idle-cycle pins and would under-approximate §6.3 gate firings — unsound
   for anti-rewrite properties if kept), and **gate-refusal semantics**
   (`update()`'s gate error is a skip-this-cycle, not a datalog halt — the
   harness must classify it, as it already does crash sentinels).

Beyond the agreed steps (candidates, not yet scheduled):

- **`AG EF success` ("no doomed states")** as a post-processing pass over the
  explored graph — the formalization of "faults never leave the success cone".
- **Scaling**: larger n / larger mixing quorums; memo growth policy;
  multi-threaded checking (the memo is already `Mutex`ed).
- **Revisits armed by faults**: the observation-timing compression becomes a
  modeling decision once rewrite faults exist (pin currency turns observable);
  stage-failure semantics in `BoardClient::post` (B1-failure = skip-and-recompute
  per the mailbox design) needs a deliberate treatment when stage faults land —
  today a transport error would surface as a halt in the harness.
- **v1 maintenance**: keep the crypto harness compiling and passing as the
  symbolic harness evolves; it re-earns its keep whenever the honest-path
  axioms are in question.
