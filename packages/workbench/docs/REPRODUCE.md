<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Reproducing the high-severity findings on the workbench

A reviewer-facing companion to [UPSTREAM_FINDINGS.md](UPSTREAM_FINDINGS.md).
Two of the recorded findings touch concerns at the root of any voting
system, so they get step-by-step, click-and-observe recipes here — enough
for a reviewer to reach the behaviour and judge it firsthand, without
reading a line of code:

- **Silent discounting** (disenfranchisement) — a voter's ballot is
  discarded with no signal that it will not count. Recorded as **S1** /
  **S2**; evidence in
  [../characterization/no-silent-discount.md](../characterization/no-silent-discount.md).
- **Spoiled-ballot choice leakage** (privacy) — a deliberately-spoiled
  (null) ballot carries the voter's candidate selection, encrypted, into
  the cast ciphertext. Recorded as **S5**.

These are **suspects, not adjudicated defects**: the recipes let you
*observe* the behaviour precisely; whether it is intended is a question for
the parties with design authority (see the framing in UPSTREAM_FINDINGS.md).
This document is deliberately kept separate from the *why* — each recipe
links back to the finding for the explanation and to the generated
characterization report for the evidence.

Each recipe states its pass condition as an explicit **iff both**
conjunction. If either half fails to reproduce, that is itself a useful
result — report it.

---

## Before you begin

**Start the workbench.** From `packages/workbench`, run `yarn dev` and open
`http://localhost:5173` (see the [workbench README](../README.md) for
prerequisites — it auto-builds `velvet-wasm` first).

**Load the demo snapshot.** Every recipe below uses one bundled fixture.
Go to **Snapshots** (`/wb`), find the row **"Explicit blank / invalid
demo"**, and click **Load**. This election has two contests:

| contest | candidates | `min_votes` | `max_votes` |
|---|---|---|---|
| Referendum (with explicit blank) | Yes · No · *Blank vote (explicit blank)* | 0 | 2 |
| Council seat (with explicit invalid) | Ada · Bruno · *Null vote (explicit invalid)* | 0 | 1 |

The two italic entries are **markers**: selecting *Blank vote* alone is a
deliberate blank; selecting *Null vote* makes the ballot explicitly invalid
(a null / protest vote).

**Set configuration with the Policy overrides panel.** Open a contest from
the left rail (**Elections → …**). Near the bottom of the contest page is a
**Policy overrides** panel exposing `min_votes` / `max_votes` as number
inputs and the six vote-validation policies as dropdowns. These overrides
are **ephemeral, per-browser-tab**: they are applied at booth-open and at
tally-run, and are **not** saved to the snapshot. To reset, click each
field's **reset**, or simply reload the page.

**Walk the booth.** Voting is reached from a voter, not the top nav: **left
rail → Voters → pick a voter → "Cast a ballot in … →"**. The booth then
steps **Start voting → (vote) → Next → (review) → Cast → Finish**. On the
voting screen, a validation problem surfaces as either an **inline warning
box directly under the question** or a **dialog when you click Next**.
"The voter was given no signal" means: **no warning box appeared under the
question, and no dialog appeared on Next** — the screen advanced straight
to review.

**Read a tally / decrypt a ballot.** From a contest page, two buttons hand
the contest's cast ballots to a sandbox:

- **Open in tally** → the `/tally` page. Click **Run tally** under
  *"2. Input ballots — array of DecodedVoteContest objects"*; the result
  appears under *"3. Output — ContestResult JSON (compute target or
  paste-in source)"*.
- **Open in ballot pipeline** → the `/pipeline` page, seeded with one row
  per cast vote, each carrying the **real cast ciphertext**. You can
  decrypt and decode it stage by stage. Used only by the S5 recipe.

---

## Part 1 — Silent discounting (S1 / S2)

**What you are checking** — the disenfranchisement condition, in two halves:

> **(no signal)** the voter is not warned by any inline message or dialog
> that their vote must be corrected, **AND (not counted)** their ballot is
> excluded from the valid total at tally (`ImplicitInvalid`).

Both recipes rely on the same enabling combination: **`invalid_vote_policy
= allowed`** removes the generic "this ballot is invalid" gate, so an
error-producing rule fires internally but nothing reaches the voter. Why
this is possible, and which rules are structurally prone to it, is in
[UPSTREAM_FINDINGS.md §S1](UPSTREAM_FINDINGS.md) and
[VALIDATION_LOGIC_DISTILLATION.md §4.5](VALIDATION_LOGIC_DISTILLATION.md).

### Recipe 1 — over-vote silently discarded

*The voter selects more candidates than allowed and is neither warned nor
counted.*

**Configure** — left rail → Elections → **Council seat (with explicit
invalid)** → Policy overrides:

- **Over-vote policy** = `allowed`
- **Invalid-vote policy** = `allowed`

**Cast** — left rail → Voters → any voter → **"Cast a ballot in … →"** →
**Start voting**. On the Council seat question tick **both Ada and Bruno**
(two selections; the max is 1). Click **Next** → **Cast** → **Finish**.

**Verify — the finding holds iff both:**

1. **No signal** — ticking the second candidate produced no warning box
   under the question, and clicking **Next** opened no dialog; the screen
   went straight to review.
2. **Not counted** — Council seat contest page → **Open in tally** →
   **Run tally** → in *"3. Output — ContestResult JSON …"*,
   `total_valid_votes` = `0` and `invalid_votes.implicit` ≥ `1`.

**Reset** — reload the page to drop the policy overrides.

Full-pipeline evidence for this exact cell:
[../characterization/overvote-e2e-pipeline.recorded.json](../characterization/overvote-e2e-pipeline.recorded.json).

### Recipe 2 — below-minimum silently discarded (includes S2)

*A ballot below `min_votes` — including a **deliberate blank** — is
discarded with no signal.*

**Configure** — left rail → Elections → **Referendum (with explicit
blank)** → Policy overrides:

- **Invalid-vote policy** = `allowed`
- **`min_votes`** = `1` or `2` (per the variant table below)

**Cast** — left rail → Voters → any voter → **"Cast a ballot in … →"** →
**Start voting**, then make the Referendum selection from the table. Click
**Next** → **Cast** → **Finish**.

| # | `min_votes` | Referendum selection | why it is below min |
|---|---|---|---|
| a | 1 | *(select nothing)* | 0 < 1 |
| b | 2 | *(select nothing)* | 0 < 2 |
| c | 2 | tick **Yes** only | 1 < 2 |
| d | 2 | tick **only** *Blank vote (explicit blank)* | the blank marker counts as 1 selection, and 1 < 2 — **this is S2** |

Variant **d** is the sharpest: it is not voter inattention but a clearly
expressed *blank vote*, silently dropped. See
[UPSTREAM_FINDINGS.md §S2](UPSTREAM_FINDINGS.md) and §S3 for why the marker
counts as a selection.

**Verify — the finding holds iff both:**

1. **No signal** — no warning box appeared under the Referendum question,
   and clicking **Next** opened no dialog.
2. **Not counted** — Referendum contest page → **Open in tally** →
   **Run tally** → `total_valid_votes` = `0` and `invalid_votes.implicit`
   ≥ `1`.

**Reset** — reload the page between variants to drop the overrides.

Full-pipeline evidence for all four cells:
[../characterization/minvote-e2e-pipeline.recorded.json](../characterization/minvote-e2e-pipeline.recorded.json).

---

## Part 2 — Spoiled-ballot choice leakage (S5)

**What you are checking** — the privacy condition:

> **(spoiled)** the voter marks the ballot null (explicit invalid),
> **AND (choice leaked)** their candidate selection is present in the
> decryption of the cast ciphertext.

This is **not** a silent discount — the voter opts into the null vote
deliberately, and its exclusion from the count is intended. The concern is
that the choice they made *before* spoiling is carried, encrypted, into the
cast ballot, recoverable by anyone who can decrypt it. The mechanism (an
asymmetry between the blank and invalid selection reducers) is in
[UPSTREAM_FINDINGS.md §S5](UPSTREAM_FINDINGS.md).

We verify this on the **ballot pipeline**, because the finding is a claim
about the *decryption*: the pipeline lets you decrypt the actual cast
ciphertext and watch the candidate fall out of the plaintext.

**Configure** — left rail → Elections → **Council seat (with explicit
invalid)** → Policy overrides → **Invalid-vote policy** = `allowed` (this
is already the baseline default; set it explicitly so the null ballot is
castable regardless of baseline).

**Cast a spoiled ballot that still names a candidate** — left rail → Voters
→ a voter → **"Cast a ballot in … →"** → **Start voting**. On the Council
seat question, tick **Ada**, *then* tick **Null vote (explicit invalid)**.
Ada stays selected — the null marker does not clear it. Click **Next** →
**Cast** → **Finish**.

**Decrypt the cast ballot** — Council seat contest page → **Open in ballot
pipeline** (it opens seeded with one row: the ballot you just cast).

1. Under *"3. Encrypted ballot envelope (HashableBallot JSON)"* — this is
   the real cast ciphertext — click the row header (**#1 ▸**) to expand it,
   then click **Decrypt ▼**.
2. *"4. Decrypted plaintext (= encoded BigUint)"* now shows **`3`**. Click
   **Decode ▼** on that row.

**Verify — S5 holds iff both:**

1. **Choice preserved in the decryption** — *"5. Decoded plaintext
   (DecodedVoteContest)"* shows `is_explicit_invalid: true` **together
   with** Ada at `selected: 0` (picked; `-1` would mean not picked).
   Equivalently, the stage-4 BigUint is **`3`**, not **`1`** — `1` would be
   the null flag alone; the extra bit is Ada's vote, recovered by
   decrypting the actual cast ciphertext. *(For a one-glance corroboration,
   the voter page also shows "Council seat → 3".)*
2. **Counts for no one** — click **Send to tally ▶** (stage 5) →
   **Run tally** → `invalid_votes.explicit` ≥ `1` and `total_valid_votes`
   = `0`. Ada was carried in the ciphertext but counted for no candidate.

**Reset** — reload the page to drop the policy override.

Recorded evidence:
[../characterization/invalid-latent-choices-e2e.recorded.json](../characterization/invalid-latent-choices-e2e.recorded.json).

---

## Automated verification

If you would rather not click through, each finding is confirmed end-to-end
(booth → encrypt → cast → decrypt → decode → tally) by a dedicated pipeline
runner — `overvote-e2e-pipeline.mjs` (S1), `minvote-e2e-pipeline.mjs` (S2),
and `invalid-latent-choices-e2e.mjs` (S5). To run all three and get one
aggregate verdict, use the orchestrator
[../characterization/reproduce-verify.mjs](../characterization/reproduce-verify.mjs).
With the dev server running (`corepack yarn workspace "@sequentech/workbench-app" dev`):

```
node characterization/reproduce-verify.mjs
```

It runs the three runners in sequence (they each reset and reload the
fixture, so they cannot overlap), checks each one's exit code and recorded
confirmed-flag, prints a `PASS` line per finding, writes
`reproduce-verify.recorded.json`, and exits nonzero if any finding is not
confirmed.

These runners set configuration via `window.__store.dispatch` and confirm the
crypto path booth-to-tally. The complementary **reviewer path** — the same
Policy-overrides panel a reviewer actually clicks — is exercised separately,
across *every* cell of the validation grid, by
[../characterization/dom-validate.mjs](../characterization/dom-validate.mjs)
(it configures through the panel on purpose), so it is no longer re-checked
here.

## Where each finding is explained

| finding | reproduce (here) | why (explanation) | evidence (generated) |
|---|---|---|---|
| S1 over-vote | Part 1 · Recipe 1 | [UPSTREAM_FINDINGS.md §S1](UPSTREAM_FINDINGS.md) | [no-silent-discount.md](../characterization/no-silent-discount.md) |
| S1/S2 below-min | Part 1 · Recipe 2 | [UPSTREAM_FINDINGS.md §S1/S2](UPSTREAM_FINDINGS.md) | [no-silent-discount.md](../characterization/no-silent-discount.md) |
| S5 choice leakage | Part 2 | [UPSTREAM_FINDINGS.md §S5](UPSTREAM_FINDINGS.md) | [invalid-latent-choices-e2e.recorded.json](../characterization/invalid-latent-choices-e2e.recorded.json) |
