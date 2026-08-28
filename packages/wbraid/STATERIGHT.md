<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Stateright model checking — braid v0.6

## What this is

braid is a re-encryption **mixnet** for elections. A small committee of
**trustees** jointly generates an encryption key such that any **threshold**
number of them (say, any 2 of 4) can cooperate to decrypt — no single trustee
can. Voters submit encrypted ballots; a designated set of trustees (the
**mixing quorum**) each **mix** the ballots in turn (shuffle them and
re-encrypt, so the order carries no information); the trustees then jointly
**decrypt** the final shuffled set. The published plaintexts cannot be linked
back to individual voters — that unlinkability is the mixnet's **privacy**
guarantee; that the output is exactly the submitted votes is its **integrity**
guarantee. Trustees do not talk to each other directly: they communicate by
posting signed messages to a shared, untrusted **bulletin board** (called *b4*).

This directory holds two test harnesses that **model-check** that protocol.
Model checking means: enumerate every reachable state of the system, exploring
all the orders in which trustees could act and all the choices an adversary
could make, and check that stated properties hold in every one of those states.
Unlike an ordinary test, which exercises a single execution, a model checker
explores the whole space of executions exhaustively. We use the
[`stateright`](https://docs.rs/stateright) explicit-state model checker.

The harnesses drive the **real** braid implementation — the real rule engine,
the real board client, real message signing — so what is checked is the code
that ships, not a separate description of it. Both live under
`crates/braid/tests/`; their module-level doc comments carry the fine-grained
design notes. This document is the map; those comments and the code are the
territory. Companion documents: `crates/braid/v0.6_spec.md` (the protocol and
its security argument) and `ASSURANCE.md` (the assurance overview).

## How braid decides what to do: the datalog

The heart of braid is a **datalog** program (a set of logical inference rules,
in the `ascent` crate). Each trustee, on each cycle, feeds its current view of
the board into these rules; the rules derive which **actions** the trustee
should now take (compute its key share, mix the ballots, sign a peer's mix,
decrypt, …) and which **errors** mean it must **halt**. The datalog is the
"brain"; a separate "action layer" performs the cryptography for a derived
action and produces the message to post. The model checkers run this exact
datalog — that is the sense in which they check the real implementation.

## Premises

Three commitments shape every design decision here.

1. **Check the real implementation.** A transition drives the real datalog and
   the real board client (with its durable committed set and its outgoing
   message record — see below). The rules are never re-expressed for the
   checker's benefit; a second copy could silently drift from the first.

2. **All uncertainty lives in the choice of action, and each action is a pure
   function.** Whatever is uncertain — the order trustees run in, whether a
   fault fires — is modeled as a *choice among the actions offered from a
   state*, which the checker explores exhaustively. Given a state and an action,
   the resulting next state is a deterministic function. Faults are therefore
   modeled as extra actions (with parameters), never as random behavior inside a
   transition. (Randomness inside a transition would make the same step produce
   different results on different visits, which breaks both exhaustiveness and
   the checker's ability to replay a failing execution.)

3. **A property is only as good as the abstraction underneath it.** For each
   (property, fault) pair we ask: does the property's truth depend on anything
   our abstraction throws away? That question decides what must be modeled
   faithfully and what may be simplified.

## The two harnesses

There are two harnesses because the cryptography creates a tension: running it
for real is faithful but expensive and non-repeatable; replacing it with
placeholders is cheap and repeatable but assumes the crypto works. We keep both.

| | `model_check.rs` — the **crypto** harness | `model_check_symbolic.rs` — the **symbolic** harness |
|---|---|---|
| Datalog, board client, message signing | real | real |
| Ballot / mix / decryption **contents** | real cryptography | deterministic **tokens** (placeholders) |
| What it checks | that the real crypto works on the honest path: honestly produced artifacts verify, and decryption recovers the inputs | everything the protocol builds *on top of* working crypto: interleavings, halting, privacy and integrity of what gets decrypted, and the whole fault program |
| Cost | high (real crypto per step) — so it is marked `#[ignore]` and run on demand | low enough for the ordinary suite; most configurations run in seconds, the two 4-trustee split-view configurations dominate at tens of seconds each (~80s total) |

How to run, from the workspace root:

```sh
cargo test --release -p braid --test model_check_symbolic      # part of the ordinary suite
cargo test --release -p braid --test model_check -- --ignored  # crypto harness, on demand
```

`--release` matters: exploration is compute-bound, and the crypto harness
especially so.

The division of labor: the symbolic harness **assumes** two facts about the
cryptography — that an honestly produced artifact passes verification, and that
a forged one does not — and checks everything that rests on those facts. The
crypto harness checks that those two facts actually hold of the real
cryptography on the honest path. An attack that breaks the assumptions
themselves (finding a proof that verifies for an incorrect shuffle, say) is a
cryptanalysis result, which no model checker of this kind could find.

Almost all of the interesting work — the fault model and the privacy/integrity
properties — lives in the symbolic harness, so the rest of this document is
about it unless noted.

## Keeping the search finite, small, and repeatable

An explicit-state checker must recognize when two executions have reached the
*same* state, so it explores that state once instead of re-exploring it down
every path that reaches it. Call this **folding**: distinct execution orders
that arrive at an identical state are folded into one node. With folding the
explored space is a **graph** (paths reconverge); without it, every distinct
order is its own branch and the space is a **tree**, which is far larger.

Three mechanisms make folding possible and the space small:

- **Deterministic contents (the tokens).** The checker recognizes "same state"
  by comparing the bytes of the state. Real cryptography draws randomness from a
  per-thread generator with no way to fix a seed, so re-computing "the same"
  mix or proof yields different bytes every time. Two orders that *should* reach
  the same state instead reach byte-different states, and folding never
  happens — which is why the **crypto** harness explores a tree. The symbolic
  harness replaces each artifact's body with a **token**: a small deterministic
  value computed from the action's inputs. Same logical artifact ⇒ identical
  bytes ⇒ the states fold.

- **Order-free state identity (canonicalization).** The board is physically an
  ordered log, but braid's logic is order-insensitive: the datalog consumes a
  *set* of facts, and a trustee's message store keys each message by the hash of
  its contents, so insertion order is irrelevant. We therefore define state
  identity to ignore board order — the board (and each trustee's records) are
  sorted into a canonical form before states are compared, and byte-identical
  duplicate rows are removed. Two interleavings that produced the same *set* of
  messages then compare equal and fold.

- **Lookahead (a determinism-and-pruning device, not a folding device).** The
  checker separates "what actions are available here?" from "what does this
  action do?". We compute each candidate action's resulting state once, cache
  it, and (a) offer only the actions that actually change something and (b)
  answer the "what does it do?" query from the cache. This makes each transition
  a genuinely deterministic lookup, which matters because when the checker finds
  a violation it reconstructs the failing execution by re-running the
  transitions — and that reconstruction must reproduce the exact states it
  checked. (Lookahead does **not** create folding: in the crypto harness the
  cached states still differ order-to-order because of the random crypto, so it
  remains a tree. Folding needs the deterministic tokens above.)

Net effect in the symbolic harness: the space is an exhaustively-explored,
order-free graph, and because the board only ever grows (messages are added,
never removed) it is acyclic — which lets us make **liveness** claims (see
Properties) that a depth-capped search could not.

## What is in a state

A state holds only what is **durable** — what survives a process restart:

- the board's committed messages (as bytes);
- the **staging area** (explained under Environment), also as bytes;
- what b4 is currently withholding from each trustee (the split-view model);
- per trustee, its two durable records: the **committed set** (every message it
  has ever admitted — braid's anti-rewrite memory, spec §6.2/§6.3) and the
  **outgoing record** (what it has posted, spec §6.4);
- fault bookkeeping and which trustees have halted.

Everything else — the live board-client object, network handles, and the fixed
**signing keys** of the trustees and manager — is *not* in the state. The keys
never change during a run, so they live on the model object (created once) and
are not part of the varying per-state data.

Because the state is exactly the durable data, every transition **rebuilds a
fresh board client from it** and runs one cycle. That is not an optimization
detail: it means every explored step is also a *restart* of the trustee from
its durable records — the very thing braid's anti-rewrite (§6.3) and
outgoing-record (§6.4) mechanisms make guarantees about. The model exercises
restart on every edge for free.

## The environment: b4, staging, and views

The untrusted bulletin board b4 is modeled at the granularity braid's fault
tolerance actually needs (spec §6.4). Posting a message is two steps:

- **stage** — upload the message body to a store b4 can read (in production, an
  S3 bucket); this is the "persist before send" step;
- **commit** — tell b4 to make the staged message visible on the board.

The state carries the staging area separately from the committed board, so the
model can express a crash *between* stage and commit. Reads are served per
trustee: each trustee sees the committed board **minus whatever b4 is
withholding from it**. With nothing withheld this is the whole board; the
withholding fault (below) is what makes trustees' views diverge — a **split
view**.

b4's only powers are to **withhold** messages and to reorder them. It cannot
forge a message (they are signed and verified), and reordering is neutralized
by the order-free state identity above. So withholding *is* the complete model
of an untrusted b4.

## Symbolic content: what the tokens carry

For the privacy and integrity properties to be checkable, the tokens are not
opaque blobs — they carry a small amount of **symbolic content** that stands in
for the real ciphertexts:

- A **ballot set** is a multiset of **voter symbols** (small integers). An
  honest shuffle is invisible at the multiset level — which is exactly the
  privacy abstraction: privacy concerns *which* ballots are present, not their
  order.
- A **mix** token records the multiset it output and a flag saying whether it
  added an **opaque layer** — an honest, adversary-unknown permutation. Privacy
  rests on every decrypted set having passed through at least one opaque layer;
  a shuffle the adversary can invert adds none.

A **strand** is one ballot set together with everything derived from it (its
mixes, signatures, decryptions) — i.e. one run of the mixnet on one input. In
the normal case there is a single strand. An adversarial manager can create a
second strand by posting a second, divergent ballot set; the split-view and
anchor material below is about what happens then.

## The fault model

Faults are **actions**. Each fault class is a variant of the per-cycle `Turn`
type; the checker explores executions with and without each fault firing, in
every interleaving. Two tiers:

- **Accidental faults are *budgeted*.** The model is configured with a maximum
  number of each (e.g. "at most 2 dropped commits"), and a per-state counter
  tracks how many have fired. Exploring every pattern of up to *k* faults yields
  a **k-fault-tolerance** claim: "under any pattern of at most *k* such faults,
  the property holds." For accidental faults we expect the strong guarantees —
  no halts, and the protocol still completes.

- **Adversarial behavior is *unbudgeted*.** An untrusted b4, a dishonest
  manager, or a below-threshold set of dishonest trustees may act freely. Here
  we cannot promise the protocol completes — an adversary can always stall it —
  so **liveness** claims are weakened to "completes, *or* an adversary acted,"
  while the **safety** claims (privacy, integrity) must hold regardless.

Two supporting disciplines:

- **Non-vacuity guards.** For each enabled fault, a companion check asserts that
  the fault *does* fire on some explored execution (and, where relevant, that
  recovery happens despite it). Without these, a fault that silently never
  triggered would let every conditioned property pass without testing anything.

- **Negative controls.** For each modeled *defense*, a dedicated test removes
  that defense and confirms the property then **fails**. This proves the
  property has teeth — that it is not passing for some accidental reason.

### The faults currently modeled

Accidental (benign):

- **Dropped commit** — a cycle whose commits never reach the board (b4 lost
  them, or the process died after committing). The message stays staged and
  recorded but is not visible. Recovery is the outgoing record (§6.4): next
  cycle the trustee re-sends the recorded message, never a recomputed one.
- **Crash before the record** — the process dies after staging but before
  writing its outgoing record. Nothing is recorded, so recovery is
  recomputation, which is safe because nothing was published.
- **Fetch failure** is deliberately *not* a fault class: a failed read aborts
  the cycle before it changes any durable state, so it is indistinguishable from
  the trustee simply not being scheduled — which the interleaving search already
  covers. (A small test pins the assumption that the read really is
  footprint-free.)

Adversarial:

- **Ballot equivocation** — a dishonest manager posts a second, divergent ballot
  set. On a consistent board every trustee that sees both sets halts on the
  collision rule (any two ballot messages conflict), which is the defense.
- **Withholding / split views** — b4 hides a whole **strand** from a trustee.
  If the trustee had already committed to something in that strand, its next
  read is missing a committed message and it halts on the **completeness gate**
  (anti-rewrite, §6.3); if it had not, it simply never sees the strand. Giving
  different trustees different withheld strands is a split view.
- **Dishonest mixers** — a below-threshold set of trustees that subvert their
  shuffle: a **known permutation** (a valid shuffle the adversary can invert —
  attacks privacy), a **forgery** (a shuffle that changes the ballot set —
  attacks integrity), or **skipping the anchor check** (below).

## The properties

We check two kinds of property. **Safety** properties say "nothing bad ever
happens" and are checked on *every* reachable state. **Liveness** properties say
"something good eventually happens" and are checked over whole execution paths —
sound here only because the exploration is exhaustive and acyclic (the board
only grows), so "eventually" is a claim the search can actually settle rather
than one it would have to give up on at some depth limit.

### Safety

The core safety properties assert the **assets** — privacy and integrity —
directly over the symbolic content, rather than asserting that particular
defense mechanisms fired. This is deliberate: one asset-level property covers
*every* way the asset could be lost, and when it fails the counterexample is the
real failure, not a proxy for it. All three hold unconditionally over the entire
fault space:

- **Privacy — differencing.** All fully-decrypted ballot sets carry the same
  multiset. Two *different* decrypted sets is the "differencing attack": the
  adversary subtracts one from the other to learn a voter's ballot.
- **Privacy — linkage.** Every decrypted set passed through at least one opaque
  (honest) shuffle, so the adversary cannot know the whole permutation and
  re-link outputs to inputs.
- **Integrity.** Any published plaintexts are exactly the legitimate ballot set.

A fourth safety property, **no trustee gets stuck**, says a trustee never halts.
It is `always`-shaped like the others (a halt never happens), but it is really
the safety companion of the completion property below: a halted trustee makes no
further progress. Under accidental faults it holds unconditionally; under
adversarial faults it is conditioned — *only an adversary may cause a halt*, a
benign fault never does.

Each defense standing behind these properties is paired with a **negative
control** (see Defenses) that removes the defense and confirms the property then
fails, proving it is not passing vacuously.

### Liveness

The liveness property is **completion**: the protocol eventually finishes —
every mixing trustee publishes its plaintexts. This is where the
accidental-versus-adversarial split matters most, and it is roughly the mirror
image of the safety picture:

- Under **accidental faults** the claim is strong and unconditional: on *every*
  path the protocol completes, whatever pattern of up-to-*k* faults fired. This
  is the **k-fault-tolerance** guarantee — the recovery mechanisms (re-send
  after a dropped commit, recompute after a crash before the record) always get
  the protocol home.
- Under **adversarial faults** completion cannot be promised: an adversary can
  always stall the protocol (an untrusted board can simply stop delivering
  messages). So the claim is weakened to "completes, *or* an adversary acted" —
  on any path where no adversary interfered, it still completes.

For each enabled fault the harness also checks **non-vacuity guards** (the
`sometimes` checks described under the fault model): that the fault actually
fires on some path, and that recovery or the expected response actually happens.
Without these a conditioned property could pass by never exercising the fault at
all.

### What is out of reach

Full privacy *as secrecy* — a statement about what an adversary can *deduce* —
is a knowledge property that no explicit-state checker expresses. The privacy
properties above are its checkable behavioral shadows.

## Defenses, and the key finding

Each modeled defense is paired with a negative control that proves it
load-bearing:

- **Honest shuffle verification.** An honest trustee signs a mix only if it is a
  valid shuffle of its input. A forged mix therefore never gathers the threshold
  of signatures it needs to extend the chain, so forged content never reaches
  decryption. (Control: make a whole quorum dishonest, so the forgery is
  blind-signed rather than rejected ⇒ integrity fails.)
- **The opaque honest layer.** With fewer than a threshold of trustees
  dishonest, at least one shuffle in the chain is honest and opaque, so privacy
  holds even if others use known permutations. (Control: make a whole quorum use
  known permutations ⇒ linkage fails.)
- **Anti-rewrite (the completeness gate and the outgoing record).** A trustee
  never acts on a board that is missing a message it previously committed to,
  and never re-sends a divergent message for a slot it already recorded.
  (Exercised by the withholding fault, whose guard confirms the completeness
  gate actually fires.)
- **The input-ballot anchor** (the key finding). braid's trustees cannot
  themselves tell a legitimate ballot set from an injected one — they process
  whatever the manager signs. So **ballot legitimacy is an external,
  publicly-verifiable fact** (the *anchor*), and the mixnet's guarantees hold
  only *given* the input is that legitimate set. Honest trustees enforce this by
  refusing to process a *first* mix whose ballots are not the anchor. The check
  lives at exactly the first-mix step — for the first mixer when it computes its
  mix, for the other quorum members when they sign it — and nowhere downstream,
  matching where the real protocol roots and signs the chain; it is
  threshold-robust (an illegitimate strand needs its *whole* quorum to skip the
  check).

  The model demonstrates *why* this anchor is necessary. Without it, an
  adversarial manager that posts a second ballot set naming a **disjoint** set
  of mixing trustees, combined with a b4 that shows each group only its own set,
  gets two strands fully processed by two disjoint honest quorums — two
  different decrypted sets, i.e. a privacy break — **with every trustee
  behaving honestly.** This becomes possible exactly when there are enough
  trustees for two disjoint quorums (committee size ≥ twice the threshold); the
  checker finds it at 4 trustees / threshold 2 and cannot construct it at 2/2.
  With the anchor check in force, honest trustees refuse the illegitimate strand
  and privacy holds; the negative control (the whole illegitimate quorum skips
  the anchor) reproduces the break. This is a modeled *honest-behavior
  assumption* — the one place we model a check braid's code does not contain,
  which is legitimate precisely because the check is inherently external and the
  assumption is now named in the trust model (`v0.6_spec.md`,
  "Input-ballot legitimacy is an EXTERNAL precondition"). A deployment must
  surface it as a per-trustee operator step: a single central check cannot catch
  a split view.

## Scope and limitations

What these results do and do not establish:

- **Exhaustive for the checked configurations — not a proof for all
  parameters.** A model checker explores the chosen configurations *completely*
  — every interleaving and every fault pattern within budget — but it is not a
  theorem prover. Our configurations are small (2–4 trustees, 2 ballots, small
  fault budgets). The results are exhaustive over exactly those, which is strong
  evidence, but they do not by themselves establish the properties for arbitrary
  committee sizes or ballot counts. Small-instance exhaustiveness is both the
  standard strength and the standard limit of explicit-state model checking.
- **The cryptographic axioms are assumed, not checked** (in the symbolic
  harness): that an honestly produced artifact verifies and a forged one does
  not. The crypto harness checks these on the honest path; breaking them is
  cryptanalysis, outside any model checker.
- **One honest-behavior assumption is modeled** — the input-ballot anchor. It is
  the single place we model a check braid's code does not perform, and it is
  legitimate precisely because the check is inherently external and is named in
  the trust model.
- **Deliberately not modeled:** privacy as *secrecy* (a knowledge property — see
  The properties), and robustness to malformed or forged messages at the
  untrusted boundary. The latter is left to the real `verify()` (which rejects
  them) and to fuzzing, which is the better tool for adversarial bytes (see
  `ASSURANCE.md`).

## Status and next steps

The benign and adversarial fault tiers are both complete, and the property set
is purely functional (privacy, integrity, plus completion/no-halt and the
guards). A per-commit log with state counts is kept separately in
`STATERIGHT-log.md`.

Not yet done, in rough priority:

- **"No doomed states."** A post-processing pass over the explored graph
  checking that from every reachable state the protocol can *still* reach a
  successful completion — the formal statement of "a fault may stall or be
  detected, but never leaves the system unrecoverable."
- **Scaling.** Larger committees and mixing quorums; bounding the lookahead
  cache's growth; parallelizing the checker (the cache is already lock-guarded).
  The general (unrestricted) split-view search is intractable at 4 trustees, so
  the current split-view tests fix the partition and search all interleavings
  within it; discovering the partition itself is a possible refinement, not
  needed for the conclusion.
- **Cost.** The two 4-trustee split-view configurations dominate the runtime
  simply because a 4-trustee committee is a much larger space than 2 or 3. The
  biggest lever would be to skip re-exploring the key-generation phase (which is
  irrelevant to the split-view attack) by starting those runs from a fixed
  post-key-generation state.
- **Crypto-harness upkeep.** Keep the crypto harness compiling and passing as
  the symbolic harness evolves; it re-earns its keep whenever the honest-path
  cryptographic assumptions are in question.
