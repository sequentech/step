# IVR System Design — Critical Review

**Scope:** the full 3,029-line spec at `ivr-system-design.md`, cross-checked
against `step/` (`sequent-core`, `harvest`, `windmill`, `admin-portal`,
`keycloak-extensions`, `Dockerfile.keycloak`, `reusable_build_push.yml`),
`beyond/`, `gitops/`, and the pre-existing `ivr-lambdas/` MVP.

## 1. Inconsistencies vs. the actual codebase

### 1.1 Invented Harvest error codes

§3.3.2, §5.4, §8.2 repeatedly refer to rejection codes `DUPLICATE_VOTE`,
`MAX_REVOTES_EXCEEDED`, `NOT_ELIGIBLE`. None of these strings exist in Harvest
or Windmill. The real `CastVoteError` enum at `insert_cast_vote.rs:135` uses
`InsertFailedExceedsAllowedRevotes`, `CheckPreviousVotesFailed`,
`VotingChannelNotEnabled`, `BallotIdMismatch`, etc. The `CastVoteRejection`
enum in the design has to map to the real variants — not to the strings the
spec invented. Also note: there is no explicit "duplicate vote" variant today;
the closest is `check_previous_votes` returning a non-success result that today
is handled via `InsertFailed`. The design's Harvest-adapter contract must be
rewritten against the real enum or the real variants must be added first.

### 1.2 Pre-existing MVP ignored

`ivr-lambdas/` already implements an IVR: two separate Lambdas
(`authenticate_voter`, `record_vote`) invoked by Connect per step, using legacy
Sequent `vote-permission-token` auth (see `authenticate_voter/src/main.rs:76`).
The design document never mentions this repo — not as "we're replacing X with
Y", not as a migration concern, not in the repo-layout section (§16.2 claims
IVR-lambda source lives in `beyond/packages/ivr-lambda/`, which doesn't exist).
That's a large omission for a technical design: whoever reads this won't know
the MVP exists, what it encodes about Connect contact-flow quirks, or that the
legacy `/vote-permission-token` flow needs deprecation.

### 1.3 EARLY_VOTING gap in authorization is real

`authorization.rs:110-114` only accepts `voting-portal` and
`voting-portal-kiosk`; `EARLY_VOTING` has no `azp` branch. The design flags
this (§C.7 footnote) but then defers it — meaning the `EARLY_VOTING` variant of
`VotingStatusChannel` is still unreachable through this function after the IVR
refactor. The enum refactor in §C.7 only covers three clients; it should cover
four, even if `EARLY_VOTING` reuses the portal client ID.

### 1.4 Kiosk client-ID drift

The design (§C.7) correctly notes the drift between `authorization.rs`
(`voting-portal-kiosk`) and `realm.rs:625` (`onsite-voting-portal`). Good catch
— but the design then says "migrate to the enum at the same time" without
specifying which string wins. That's a real decision with wire-level
consequences (existing realms ship one or the other) and should be nailed down,
not hand-waved.

### 1.5 I18nContent type mismatch is acknowledged but the chosen fix leaks typing

`ballot.rs:40` pins `I18nContent<T = Option<String>>` and presentation fields at
`:454/:966/:1274` use `I18nContent<I18nContent<Option<String>>>`. The design
(§7.2 type-system note) correctly identifies that an `i18n[lang]["ivr"]` object
can't fit a `String` leaf and chooses the "untyped escape hatch". Consequence
not called out: the admin-portal editor (§7.4) and the sanitizer (§7.2 SSML)
both have to consume the IVR sub-tree as `serde_json::Value`, which defeats the
"typed dispatch" selling point and makes the published shape silently diverge
from what the Rust types describe. The "widen leaf type" option is deferred to
nowhere — there is no ticket or sequencing.

### 1.6 VotingChannels.telephone already exists

`core.rs:215` already has the `telephone: Option<bool>` field. The design's
Appendix C.2 says to add a `TELEPHONE` arm to `channel_from()` — the arm to
read this existing field — which is correct but gives the impression that the
channel flag itself is a code change. It isn't. Clarify.

### 1.7 Paths that don't exist are cited like they do

- `beyond/packages/ivr-lambda/` (§16.2, §16.3.1) — not present;
  `beyond/packages/` contains only `ballot-audit/`.
- `beyond/packages/keycloak-extensions/ivr-config-resource/` (§16.2, §C.8.2) —
  not present; `beyond/` has no `keycloak-extensions` tree at all. The existing
  `conditional-authenticators`, `message-otp-authenticator` etc. live in
  `step/packages/keycloak-extensions/`. Moving Keycloak extensions across repos
  is a real decision that hasn't been made yet.
- `beyond/packages/ivr-contact-flows/` — not present.
- `gitops/iac-aws/ivr/<env>/` — not present; `gitops/iac-aws/` has `cluster`,
  `rds`, `vpc`, `vpc-peering` patterns.
- `gitops/unified/global-config-apps/ivr/phone-map.yaml` — not present; that
  dir holds `harvest`, `hasura`, etc.

The spec reads as if these are conventions; they're aspirational. Mark them as
such or §16.2 risks being read as documentation of existing state.

### 1.8 packages/keycloak-extensions/ivr-dob-authenticator/ path in §16.2

Shown once as `packages/keycloak-extensions/ivr-dob-authenticator/` (no
`beyond/` prefix), once as
`beyond/packages/keycloak-extensions/ivr-dob-authenticator/` (§16.3.2). Pick
one.

## 2. Design issues (flow, state, consistency)

### 2.1 Amazon Connect invocation model vs. the "exactly one phase per invocation" claim

§3.5.3 says "every invocation loads the session … executes exactly one phase …
saves … responds." But the Connect flow diagram in §12.1 has four distinct
Lambda invocations per turn (`ProcessStep`, `ProcessInput`, `HandleTimeout`,
`HandleError`) plus `InitSession`. Those are separate Lambda invocations, and
only one of them (`ProcessStep`) is a no-input advance. In practice,
`HandleTimeout` and `HandleError` must also mutate `RetryCounters` and may
themselves terminate a phase. The §3.5.3 description is oversimplified — a more
honest model is one phase-engine turn per Lambda invocation, where "turn" is a
specific input category. Without that distinction the handler will either have
four branches that all look the same or will accidentally collapse timeouts
into invalid-input handling.

### 2.2 Concurrency model has a hole §4.1 doesn't address

§4.1 justifies optimistic concurrency ("DTMF captured at the tail of one prompt
can land during the next `ProcessStep` call"). But the remediation —
`SessionRaced` → "please try again" — is a voter-hostile outcome in exactly the
case the blockquote describes. If two invocations legitimately contend and one
loses, the losing invocation's DTMF (which the voter did press) is dropped.
"Please try again" isn't a retry; it's asking the voter to re-press, which they
won't know to do because from their perspective Connect already consumed the
input. Options the design should pick among: (a) serialise via Connect's
contact-flow (no parallel invocations) — but this contradicts §4.1's premise
that Connect can overlap, (b) queue the late input on the session row and
replay it on the next turn, (c) ignore and log. "Please try again" is the worst
of the three and isn't even well-defined at the prompt level (what does
`SessionRaced` play?).

### 2.3 ballot_id caching correctness is real, but the cache has a race

§4.1 requires the encrypted ballot + `ballot_id` to be cached per
`(session, election)` on first `ElectionSubmit`. Good. But note the SHA-256
confirmation at `insert_cast_vote.rs:495-508` is computed over the deserialized,
re-serialized `HashableBallot`, not over the raw content bytes. If the Lambda
caches content as a `String` and retries with that exact string, but the
encoding round-trip isn't byte-stable, the hash will drift. The design asserts
"identical resubmission hashes to the same `ballot_id`" — verify this
empirically with a test that round-trips the cached content through
`hash_ballot` and confirms determinism. Also: the deduplication path is
`BallotIdMismatch` or `InsertFailedExceedsAllowedRevotes`, not a clean "this
`ballot_id` already exists" — the design should map the actual rejection path,
not an assumed one.

### 2.4 Re-entrant voting reconstruction endpoint doesn't exist

§9.3 says "a Harvest listing endpoint that returns already-cast ballots for
`(voter_id, election_event_id)`". There's no such endpoint today.
`check_previous_votes` is an internal function (`insert_cast_vote.rs:967`), not
a REST route. This is a cross-cutting feature (needed by the portal too,
arguably) — the design should either add it to scope with a specified signature
or identify why it's out of scope and what the voter sees when they redial.

### 2.5 skip_election_list + already-voted elections = dead state

If `skip_election_list=true`, there's only one election, and the voter already
cast it on a prior dropped call, §9.3 says the selection UI renders "already
voted". But §3.3.2 says `ElectionSelect` is skipped when `skip_election_list`
=true and only 1 election — which means the voter enters `LanguageSwitch` →
`ElectionIntro` → `ContestLoop` → … for an already-closed election. The
re-entrant protection is in `ElectionSelect`, which is precisely the phase
being skipped. Either (a) the skip must be conditional on "not already voted",
or (b) `ElectionSubmit` must handle the `CheckPreviousVotesFailed` case
gracefully without re-encrypting.

### 2.6 VoteConfirm → ElectionSummary navigation back is underspecified

§3.3.4 introduces `election_list_skipped` on the cursor so
`VoteConfirm`/`ElectionSummary` can decide whether to offer "back to election
list". But the sub-phase table (§3.3.3) makes no mention of this branch —
`VoteConfirm` is documented as confirm/change only. Either the cursor field is
unused or the UX is missing from the table.

### 2.7 Contest-edit semantics for multi-select

§3.3.3 `ElectionSummary` says "editing a contest clears its selections and
re-enters `CandidateSelect` for that contest only". For a `max_votes>1` contest
with 5 selections made, the voter now has to re-select all 5. That's the right
behavior — but it collides with the `pending_selections` accumulator field
(§3.3.4): the cursor must clear both `votes[contest_id]` and
`pending_selections`, which the design calls out in a single sentence. Make
this an invariant, not a note; every path that enters `CandidateSelect` via the
edit code-path must run through one helper that resets both. Otherwise the
first forgetful refactor leaks selections from the pre-edit state.

### 2.8 Language-switch scope is leaky

§3.3.3 `LanguageSwitch`: "switch affects prompts for this election only." But
`session.language` is a single field (§4.1). For a multi-election call where
election A is bilingual and election B is unilingual, the Lambda either
(a) restores the event-level `session.language` after leaving election A, which
requires tracking the pre-switch language per election frame — not currently on
the cursor, or (b) lets the language "sticky" into election B. The design
implies (a) but doesn't carry the state to make it work.

### 2.9 announcement with accept_key + *="repeat" contradiction

§3.2 lets `announcement` phases wait for an acceptance key (e.g., `2` for
declaration). §3.4 reserves `*` uniformly as "repeat". What if the declaration
prompt is playing and the voter presses `*`? The reserved-key table says `*`
repeats; the phase's `accept_key` says only `2` advances. If `*` is genuinely
reserved, every `announcement` phase needs a free repeat branch; document it in
the phase executor, not just in the reserved-key table.

### 2.10 0="skip/abstain" + max_votes > 1 contest

§3.4 says `0` means "skip/abstain this contest". In a multi-select contest,
after the voter has already pressed candidate `3`, what does `0` do? Skip
(discarding `3`) or blank-vote (invalid because a selection exists)? Not
defined. The table lists the key's meaning but not its interaction with
in-progress accumulation. Define once and enforce.

### 2.11 ElectionSummary input grammar collides with single-digit contest counts

§3.4 says summary uniformly uses multi-digit (`00#` / `NN#`). Good rule. But
§3.3.3 says voters press `00#` to submit — a voter habituated to single-digit
input elsewhere in the same call will press `1` (expecting "submit") and get an
`invalid_input`. The design defends uniformity ("one rule, no edge cases") but
the cost of that uniformity is an invisible failure mode at exactly the moment
of submission, which is the highest-stakes turn in the whole call. Consider a
softer alternative: accept either `00#` or `1#` or leverage Connect's
configurable terminator. At least call out the UX cost.

### 2.12 # is both terminator and "end multi-select" — overload

§3.4 gives `#` two jobs: "terminator for multi-digit input" AND
"end accumulation in a multi-select contest". These collide in a multi-select
contest that uses multi-digit candidate codes (`>9` candidates). The voter
presses `05#07#03#` — is the first `#` a terminator for `05` or an "end"
signal? The answer has to be "terminator while mid-code, end-of-accumulation
when cursor is empty after a terminator", which is fine but subtle enough that
it needs explicit specification and tests. Currently it's a sentence.

### 2.13 Session-idempotent session creation

§3.5.5 step 2: "Load or create the session via the Session port." Two
concurrent cold-start invocations for the same `contact_id` (which happens with
Connect retries) will both try to create; the optimistic-concurrency guard
applies to writes but not to reads. The init path needs a specific
"create-if-absent with `ConditionExpression: attribute_not_exists(contact_id)`"
— not mentioned. Without it, the second concurrent creator silently wins.

### 2.14 Per-phone-number routing table is a routing oracle

§6.2 `PhoneConfig` stores `keycloak_url`, `harvest_url`, `hasura_url`,
`s3_public_base_url` per row. A compromised or mis-typed row redirects calls to
arbitrary URLs. Two defenses:

- Signed routing table: the DynamoDB row itself should be write-gated by a
  narrow IAM policy (not the Lambda's execution role). The design says
  "read-only from the Lambda" but doesn't say who writes. `gitops` does
  (§16.2), which is fine — but the design should spell that out and ban write
  from the Lambda role.
- URL allowlist: the Lambda should compare the resolved URLs against an env-var
  allowlist of trusted hostname suffixes (`*.sequentech.io`) before using them.
  Belt-and-suspenders.

### 2.15 blacklist_check runs before authentication, but with a service JWT

§6.3 has the Lambda obtain a service-account JWT via Keycloak's
client-credentials grant against the tenant's realm at cold-start. Two
problems:

- Cold start now requires a Keycloak round-trip, which adds latency to the first
  call after a scale-out. Benchmark.
- A multi-realm Lambda needs a service token per tenant realm (§6.2 routes by
  dialled number → tenant → realm). "Cache it like any other bearer token"
  elides that the cache key is `(tenant, realm)`, not a single token. And
  tokens expire — the refresh path must work under the same `TokenManager`
  abstraction but with client-credentials grant, which behaves differently from
  ROPC refresh. Spell out the port signature.

### 2.16 Salt for phone-hash rotation has an implementation gap

§9.2.1 says "never rotate the salt mid-election" but election events run in
parallel across tenants in a shared Lambda. There is always some tenant
mid-election. "Never rotate" therefore means "never rotate at all, ever."
Either the salt is per-tenant (expensive — one Secrets Manager read per cold
start per tenant) or the rotation window must be coordinated with the
platform's tenant calendar. Specify which.

### 2.17 assistance_phone interpolation is a cross-language pitfall

§7.3 shows `auth_max_attempts` with `{assistance_phone}`. §7.2 SSML renderer
escapes all `{var}` slots. A phone number `1-800-555-0199` has no XML-breaking
characters today, but `{assistance_phone}` is listed in Appendix D as a
template variable, which means the renderer will XML-escape it. Correct, but
harmless only so long as phone numbers never contain `&` or `'`. Fine — but
while you're there, add a test that the escape pass is idempotent on the known
placeholder set.

## 3. Hexagonal architecture review

### 3.1 Port surface is right-sized, one caveat

The 7 ports (§3.5.2) cleanly separate external dependencies. The "Blacklist"
port is implicitly a second use of Hasura (same backend, different JWT,
different permission). Making it a distinct port is correct — two different
access patterns — but calling out that it shares the adapter implementation
under the hood would prevent duplicate-client pitfalls (connection pooling,
retry budget, etc.).

### 3.2 Domain/adapter seam leaks at the SSML renderer

§7.2 puts the SSML renderer in `sequent-core` (so admin-portal WASM and the
Lambda share it). That's right. But the renderer needs tenant/election context
to do the Polly preview in the admin portal, and `sequent-core` shouldn't
depend on Polly. The preview is a separate concern — the renderer outputs
sanitized SSML text; Polly synthesis is an adapter call. The design conflates
them in the "admin-portal editor requirement" paragraph. Split: sanitizer in
`sequent-core` (pure); Polly-preview adapter in `admin-portal/backend`.

### 3.3 "Phase ports aggregate" is fuzzy

§3.5.3 says "however it's expressed — a trait, a struct of references, a
context object". That flexibility is fine as aspiration but a design doc should
pick one. Rust's trait-object vs. struct-of-references has real ergonomic
consequences (object-safety for `dyn Port` vs. monomorphization, test-double
story, etc.). Pick one before engineers implement three.

### 3.4 Sub-phase dispatch is a second flow engine — should it reuse the outer one?

§3.5.4 treats the ballot loop as a mini flow engine. That's conceptually right
but the two dispatchers (outer and inner) are not unified in the design. Either
(a) generalize: one typed-enum dispatcher parameterized by phase-variant, reused
at both levels, or (b) accept the duplication. If (b), document why —
otherwise every phase type added to the outer pipeline will re-litigate whether
the inner loop should adopt the same mechanism.

### 3.5 PhaseState variant coupling to FlowPhase

§4.1 requires `PhaseState` and `FlowPhase` variants to match positionally.
Expressed as a runtime invariant ("the engine enforces this"). Typed dispatch
would make this a compile error: parameterize `FlowPhase` over its state
(`enum FlowPhase { Announcement(AnnouncementConfig, AnnouncementState), … }`).
Worth doing; the design's current shape defers it to runtime.

## 4. Security review

### 4.1 /ivr-config endpoint is public and leaks auth-shape

§5.1.2: "The endpoint is public (no auth required). The shape of auth steps is
not sensitive — voters already know what to enter." Partially true — but it
also leaks which tenant realms exist, whether OTP or DoB is in play, and
(critically) `max_digits` and `maps_to` for custom authenticators. An attacker
enumerating tenants across realms learns the auth scheme of each. The marginal
security cost is low, the marginal engineering cost of requiring the service
JWT (the same one §6.3 already introduces for blacklist) is also low. Consider
requiring it.

### 4.2 Refresh-token-in-DynamoDB threat model is thin

§9.2 says "Store `refresh_token` securely in DynamoDB (encrypted at rest)."
DynamoDB at-rest encryption is SSE by default; that's table-level encryption,
not row-level or KMS-customer-managed. If an engineer with DynamoDB read access
can read the refresh token, they can impersonate any active voter. Either
(a) encrypt refresh tokens at the application layer with a KMS CMK keyed per
tenant, or (b) accept the threat and document it. Current wording implies
stronger protection than SSE-S3-style encryption actually provides.

### 4.3 Phone-number in DynamoDB session + TTL

§9.2.1: raw E.164 phone lives in the session row for up to 1h idle / 10h max.
DynamoDB TTL is best-effort — AWS says deletion happens within 48h of expiry,
not at expiry. That breaks the 1h retention story by up to 48h. Add a scheduled
Lambda or batch job that sweeps expired rows on the actual 1h boundary, or
document the reality.

### 4.4 Connect contact-flow retry loop is a voter-side safety invariant

§4.1 declares "the Connect contact-flow MUST be authored so that `ProcessInput`
is not re-invoked on handler timeout. … asserted in the contact-flow fixture
tests". Connect contact flows are authored via AWS Console OR JSON
export/reimport. The assertion test must parse the Connect JSON and check that
the Invoke Lambda block's `ErrorHandler` doesn't loop back to the same
`GetCustomerInput` block. That's a real parser, not a stub. Specify where it
lives — `step/`, `beyond/`, or the gitops-level IaC apply-time check.

### 4.5 Brute-force via hang-up-and-redial

§9.3 relies on `Keycloak bruteforceProtected=true`. Two points:

- Keycloak's brute-force protection locks the user, not the caller. An attacker
  with 10 phone numbers and 1 target voter ID still hits `failureFactor` fast,
  so this works for the voter-ID-guess case but not for a spray attack against
  many voters.
- The alert "more than 5 calls from the same hash within 30 minutes" uses the
  salted hash. After a salt rotation (quarterly, per §9.2.1), the counter
  resets. Callers who cross a rotation boundary escape the detection window.
  Low-priority, but worth acknowledging.

### 4.6 Vote secrecy and contact_id linkability

The CloudWatch log carries `contact_id` on every turn, plus phase events, plus
`VoteRecorded`/`VoteSubmitted`. A CloudWatch reader can reconstruct the ordered
sequence of candidate selections per `contact_id`, and the salted phone hash
correlates sessions across calls (§9.2.1). Vote secrecy on the wire is intact
(encrypted ballot); vote secrecy in the operational log is weaker than it needs
to be. Consider: never log candidate-level selection events; log only
"selection made" and "ballot submitted" aggregated per contest, so the log's
information-theoretic reach is bounded.

### 4.7 SSML sanitizer is a genuine trust boundary — good call

§7.2 correctly identifies this and ships a real design (allowlist, break time
cap, `{{ssml:var}}` recursion bound, fail-loud on malformed XML). Two gaps:

- The `xml:lang` allowlist is static (`en-CA`, `en-US`, `fr-CA`, `fr-FR`). A
  Canadian deployment with First Nations languages needs to change code to add
  locales. Make it configuration.
- Polly supports `<prosody rate="…">` which the design explicitly strips. A
  legitimate use case — slowing down candidate readback for elderly voters —
  can't be met without code change. Consider adding `prosody` with a narrow
  attribute allowlist (`rate: x-slow, slow, medium, fast`; no pitch, no volume,
  no `%` values).

### 4.8 IVR Lambda's IAM execution role scope

§11.1 says the Lambda has DynamoDB, Secrets Manager, VPC access. What else?
Implicit (via VPC) — egress to Keycloak/Hasura/Harvest/S3. Not listed
explicitly: CloudWatch Logs (`logs:CreateLogStream`, `logs:PutLogEvents`) and
the fact that the role needs read-only DynamoDB access to `ivr-phone-config`
but read-write to `ivr-voting-sessions`. Separate the two tables under
different IAM statements — one compromise of the Lambda shouldn't let an
attacker rewrite phone-number routing.

### 4.9 No rate limiting at Connect → Lambda boundary

§10.3 has `IvrConnectConcurrentCallsNearQuota` at 80% of the service quota but
no per-phone-number call-rate cap. A DDOS via a spoofable robodialler against
one election's DID could burn the Connect concurrency quota for every other
election sharing the same Connect instance. Consider per-DID concurrency caps
(Connect supports them) and document the choice.

## 5. Deployment / ops review

### 5.1 Multi-cluster routing puts the Lambda in the critical path for every tenant

§6.2 + §11.2 design a single Lambda deployment that dispatches to every
cluster. Elegant, but: one bad deploy of the Lambda takes down IVR for every
tenant simultaneously. Compare to per-cluster portals, which degrade
independently. Mitigations: canary deploys, active traffic shifting via Lambda
aliases, per-tenant feature flags to disable IVR without code roll. The design
has no blast-radius or rollback discussion.

### 5.2 Single region, Connect's own AZ failure is ignored

§11.2 says Lambda is single-region. Amazon Connect instances are also
single-region; if `ca-central-1` has an impairment, the entire IVR is down for
every tenant on election day. No DR story — no "warm-spare Connect instance in
`us-east-1` with DNS-based failover" or equivalent. For a public-election
channel that's a notable gap; document the risk even if DR isn't in scope now.

### 5.3 Gitops layout proposal is speculative

§16.2 asserts paths (`gitops/iac-aws/ivr/<env>/`,
`gitops/unified/global-config-apps/ivr/phone-map.yaml`) that don't exist and
don't match existing gitops shapes. Current layout:
`iac-aws/{cluster,rds,vpc,vpc-peering}` and
`unified/global-config-apps/{harvest,hasura,keycloakx,…}`. Propose either
(a) `iac-aws/ivr/` (new top-level, consistent with `iac-aws/rds/`) or
(b) `iac-aws/cluster/ivr.tf` (nested). The design picks neither. The
`unified/global-config-apps/ivr/phone-map.yaml` path is also non-standard —
current apps are Argo Applications referencing Helm charts, not raw YAML config
files.

### 5.4 beyond-as-source-of-truth is an unverified architectural decision

§16.2 makes a significant claim: IVR Lambda source lives in `beyond`, not
`step`. `CLAUDE.md` is silent on this — `beyond/` currently hosts
`ballot-audit` (a separate tool) and IaC, not Lambda code that `step` compiles.
The design proposes cross-repo Cargo workspace inclusion ("pulled into step's
Cargo workspace as a workspace member (via a path reference from the `beyond`
checkout, or a vendored/submoduled include)"). That's a large CI/build-graph
change — submodules across `step` + `beyond` + `gitops` have non-trivial
ergonomic costs (Atlantis CI-hosts, dev-container caching). The design hedges
("submodule, sparse clone, or whatever mechanism `step` adopts") — pick one. If
the answer is "we haven't decided yet", this section needs to move to "Open
Questions".

### 5.5 Keycloak extension ivr-config-resource location vs. existing keycloak-extensions tree

The existing extensions all live in `step/packages/keycloak-extensions/` and
are built into the keycloak Docker image via `Dockerfile.keycloak`. The design
proposes the IVR extension live in
`beyond/packages/keycloak-extensions/` and is pulled back into the `step`
Keycloak image at build time. That's a cross-repo Java build for symmetry with
the cross-repo Rust build. Simpler alternative: put the IVR extension next to
the others in `step/packages/keycloak-extensions/ivr-config-resource/` and skip
the cross-repo dance entirely. The stated rationale — "lives alongside
conditional-authenticators and other extensions already packaged into the
Sequent Keycloak image in beyond" — is factually wrong; they're packaged in
`step`. Re-examine.

### 5.6 CloudWatch → Alertmanager bridge is genuine work, not a checkbox

§10.3 says "CloudWatch alarms by themselves do not reach the cluster
Alertmanager" and picks "CloudWatch exporter → Prometheus scrape" as the
recommended path. That's real work: running `cloudwatch-exporter` or `YACE` as
a scraped target in the infra cluster with IAM credentials for cross-account
CloudWatch read. Not sized, not scheduled, not owned. For an MVP, falling back
to SNS + Alertmanager webhook is simpler; the design should commit and ship
Option 2 as a later improvement.

### 5.7 IvrNoCallsDuringElection dead-air canary needs a baseline

"`ivr.calls.total == 0` for 30 min while `telephone_voting_status` is `OPEN`".
For a small municipality during a low-turnout hour, zero calls for 30 min is
normal. The canary will false-fire off-peak. Tune by expected baseline
per-election or make the threshold configurable per-event.

### 5.8 Retry budget vs. Keycloak client lockout interaction

§8.1 has per-call `retries.auth = 3` before disconnect. §9.3 has Keycloak
`bruteforceProtected` with `failureFactor` — typical default is 30. So a voter
who truly doesn't know their PIN gets 3 tries per call, can redial, and rack up
30 failures before getting locked out. That's 10 calls of frustration. The two
budgets aren't integrated: the Lambda has no way to know the voter is on
attempt 29 of 30 on a redial. Consider (a) surfacing
`account_temporarily_disabled` as a dedicated prompt before the voter exhausts
the per-call budget on redial #10, and (b) coordinating the Keycloak
`failureFactor` with the §8.1 budget so they line up.

### 5.9 No migration or data-retention plan for blacklist between election events

§6.3 says blacklist rows may be scoped to an event or tenant-wide
(`election_event_id` nullable). What happens to event-scoped blacklist rows
after an event ends? Auto-delete? Retain for audit? Undefined. Will come up in
a PIPEDA review.

### 5.10 Cost model glosses over Connect concurrent-call quota

§17 sizes the 50K-voter case at ~42 concurrent calls, "well within default
Connect + Lambda concurrency limits." Amazon Connect's default concurrent-call
quota per instance is 10 for new instances; you request increases. For any real
deployment, §17 needs a "contact AWS before go-live to raise the quota to X"
line item. Otherwise the first election-day spike trips the quota.

### 5.11 NAT Gateway multi-AZ is recommended but undecided

§11.1 says "Decide before Phase 3" — a genuine open question marked as such,
which is fine. But the cost model (§17) uses the multi-AZ figure. Align: either
commit to multi-AZ everywhere or show both scenarios.

## 6. Testability & process

### 6.1 Text-in / text-out harness is a strong choice

§15.2.1 (`step-ivr` CLI + hosted preview) is one of the better parts of the
design — the pure-function shape of the engine makes this nearly free, and it
collapses three concerns (unit tests, reproducing prod issues, admin-portal
flow preview) into one surface. Keep it in scope.

### 6.2 /ivr-config contract test spans repo boundaries

§15.3 "spin up Keycloak with a representative Direct Grant flow and assert the
`/ivr-config` response shape." If the Java extension lives in `beyond` and the
Lambda that parses the response lives in `beyond` (both per §16.2), the
contract test needs to run in a CI job that has both. In practice the cheapest
place is `step` (which already runs `mvn verify` for Keycloak extensions via
`java_test.yml`). Commit the location.

### 6.3 No load-test sizing

§15.5 "concurrent call simulation" is a heading. What's the target? 42
concurrent calls (§17.4)? 500 for a large municipality? The answer drives
Lambda-reserved concurrency, DynamoDB capacity, Connect quota. Give a number.

### 6.4 Record-and-replay is only as good as the prompt-key assertions

§15.2 asserts `expected_prompt_key`. Fine, but §7.2 lets overrides inject SSML.
A regression in the SSML renderer that produces wrong audio but the right key is
invisible to this harness. §15.2 footnote ("assert the final sanitized SSML
string, not just the prompt key") is correct; elevate to the main text so it
isn't skipped in implementation.

## 7. Summary — highest-priority items

- Harvest rejection codes don't exist — rewrite §5.4, §3.3.2, §8.2 against the
  actual `CastVoteError` variants in `insert_cast_vote.rs` before implementation
  starts.
- Pre-existing `ivr-lambdas/` MVP is not acknowledged — add a deprecation /
  migration section; spell out what breaks, who owns the cutover.
- Repo-location claims in §16.2 are aspirational — most `beyond/…` paths don't
  exist; decide whether the Lambda + Keycloak extension actually move, document
  the cross-repo build story concretely, and stop referring to proposed paths
  as though they're current state.
- `SessionRaced` UX is wrong — a dropped DTMF cannot turn into "please try
  again"; the voter already pressed a key. Pick queue-and-replay or
  silent-drop-with-log.
- `skip_election_list` + already-voted-on-redial bypasses protection — the
  re-entrant guard lives in `ElectionSelect`, the phase that gets skipped. Fix
  in §3.3.1.
- `I18nContent` type mismatch — the untyped-escape-hatch choice defeats the
  typed-dispatch rhetoric elsewhere. Schedule the leaf-widening refactor or own
  the trade-off.
- Keycloak kiosk client-ID drift (§C.7) — pick `voting-portal-kiosk` or
  `onsite-voting-portal` and migrate realms; don't just note the drift.
- Cross-tenant brute-force, vote-secrecy in operational logs, refresh-token
  protection — three security items that need explicit threat-model paragraphs,
  not scattered mitigations.
- CloudWatch → Alertmanager is real work — size it or ship SNS-webhook first.
- `/ivr-config` public endpoint — trivially upgradeable to service-JWT-gated; do
  it.

The doc is strong on domain decomposition (flow engine, sub-phases,
reserved-key uniformity, SSML sanitizer) and weaker on code/system archaeology:
it treats proposed paths as existing, invents error codes, and doesn't grapple
with the MVP already in production. A pre-implementation pass reconciling the
spec with `step/`, `beyond/`, `gitops/`, and `ivr-lambdas/` would save most of
the above.
