---
id: windmill_approval_matrix
title: Enrollment approval matrix
sidebar_position: 3
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Enrollment approval matrix

When a voter enrolls, Keycloak's `LookupAndUpdateUser` authenticator posts the applicant data to
Harvest's `/verify-application`, and `verify_application` in
`packages/windmill/src/services/application.rs` decides the application with the election event's
approval matrix. The [Approvals reference](../../02-election_managers/02-reference/02-election-event/14-election_management_election-event_approvals.md)
describes the feature for election managers.

## Model

`packages/windmill/src/services/approval_matrix/mod.rs`:

```json
{
  "compared_fields": ["firstName", "lastName", "dateOfBirth"],
  "rules": [
    {"when": {"already_enrolled": true}, "then": {"decision": "REJECTED", "reason": "ALREADY_APPROVED"}},
    {"when": {"identity": "MANUAL_ENTRY"}, "then": {"decision": "PENDING", "reason": "IDENTITY_NOT_VERIFIED"}},
    {"when": {"differing": "none"}, "then": {"decision": "ACCEPTED"}}
  ],
  "otherwise": {"decision": "PENDING", "reason": "NO_VOTER"}
}
```

| Key in `when` | Values | Meaning |
|---|---|---|
| `identity` | `VERIFIED`, `MANUAL_ENTRY` | How the identity was established |
| `voter_found` | `true`, `false` | The registry returned a voter |
| `already_enrolled` | `true`, `false` | One of the `unset-attributes` is set on the voter |
| `valid_id` | A string | The applicant's `sequent.read-only.id-card-type` |
| `differing` | `none`, `exactly_1`, `at_most_1`, `exactly_2`, `at_most_2`, `at_least_3` | Compared fields that differ |
| `fields` | `{"<field>": "MATCHES" \| "DIFFERS"}` | A compared field's result |

`then.decision` is `ACCEPTED`, `PENDING` or `REJECTED`; `then.reason` is `NO_VOTER`,
`ALREADY_APPROVED`, `INSUFFICIENT_INFORMATION`, `IDENTITY_NOT_VERIFIED` or `OTHER`. `EMAIL_IN_USE` is
not a rule's: `verify_application` rejects an accepted enrollment with it when the flow's
`update-attributes` write the email and another voter of the realm already has it, unless the realm
allows duplicate emails.

Another voter can take the email after windmill accepts the enrollment and before Keycloak updates
the voter. Keycloak then checks again and asks harvest's `/reject-application-email-in-use`, as the
tenant's service account, to reject the application. Harvest locks the application and rejects it
only if it is still automatically accepted, was sent from the asking enrollment session (stored as a
digest), matched the named voter, writes the email, and the email still belongs to another user of
a realm with unique emails, with the voter not enrolled. Asking again once rejected succeeds, so a
lost answer can be retried. Until harvest answers, the voter sees an error and a reload asks again;
only then does Keycloak show the rejection and send it. The two databases still commit apart:
another voter can take the email after Keycloak's own check, and Keycloak's update of the voter then
fails as before, with the application left accepted.

Unknown keys and values fail deserialization. `ApprovalMatrix::validate` returns the errors that keep
a matrix from being saved: `NO_COMPARED_FIELDS`, `DUPLICATE_COMPARED_FIELD`, `UNKNOWN_FIELD` (a rule
on a field that is not compared), `ACCEPTS_MANUAL_ENTRY`, `ACCEPTS_ALREADY_ENROLLED`,
`ACCEPTS_WITHOUT_VOTER`, `OTHERWISE_ACCEPTS`, `MISSING_REASON` and `UNEXPECTED_REASON`.

## Evaluation

`approval_matrix/evaluate.rs`:

- `decide(matrix, inputs)` returns the first rule that applies to one set of `RuleInputs`, or
  Otherwise. A field missing from the inputs is not compared for the identity document: it counts as
  differing in `fields` conditions and is not added to `differing`. Rules with `differing` or
  `fields` never apply without a voter.
- `evaluate(matrix, identity, valid_id, candidates)` calls `decide` for every registry voter the
  lookup returned and combines the results: one accepted voter accepts; several go to manual review;
  otherwise any pending voter does; otherwise the enrollment is rejected, preferring a voter who is
  already enrolled. Candidates are sorted first, so the result doesn't depend on their order.
- The invariants are enforced after the rule is found, whatever the matrix says: no acceptance
  without a voter, of an already enrolled voter, of an identity entered manually, or by Otherwise.
  The `invariant` of the decision names the one that applied.

`ApprovalMatrix::built_in` is version 1, used by events without a saved matrix. Its fixtures in
`evaluate_tests.rs` compare it, for every combination of differing fields, with the decisions the
fixed branches of `automatic_verification` took before.

The compared fields of a saved matrix replace the `search-attributes` of the authenticator, both for
the registry lookup and for the comparison. Without a saved matrix, `search-attributes` is used.

### Identity method

An identity verification step reports how it established the identity by setting the
`identity-method` auth note to `VERIFIED` or `MANUAL_ENTRY`. `LookupAndUpdateUser` forwards it in the
annotations of the application. When no step sets it, `identity` conditions don't apply.

Only Keycloak sets the note:

- The registration form can't: `identity-method` is a reserved note, and submitting the form again
  removes the note an earlier verification set.
- The Scanovate step sets `VERIFIED` only when it completes a verification: after the voter confirms
  the attributes it read, or right away when it stores none. Starting the step over (a reload, a
  retry, the attempt limit), a failed check or a service error removes it, so a verification never
  carries over to a later failed attempt.
- A voter whose status attribute says they were verified before skips the Scanovate step without a
  method: the attribute is not proof of a verification in this session.
- `LookupAndUpdateUser` forwards only `VERIFIED` or `MANUAL_ENTRY`; it ignores any other value with a
  warning, and windmill refuses an application whose annotations carry one.

No step of the platform sets `MANUAL_ENTRY` yet: an enrollment without a verification step carries
no method.

## Versions

Table `sequent_backend.approval_matrix` (`tenant_id`, `election_event_id`, `version`,
`compared_fields`, `rules`, `otherwise`, `sha256`, `created_at`, `created_by`,
`created_by_username`). A trigger refuses updates. Hasura allows `application-read` and
`approval-matrix-write` to select and nobody to write.

`approval_matrix/store.rs` reads the latest version, saves the next one (the built-in matrix counts
as version 1, so the first save is version 2) and imports the matrix of an election event bundle as
version 1.

## Actions

| Action | Harvest route | Permission | Does |
|---|---|---|---|
| `get_approval_matrix` | `/get-approval-matrix` | `application-read` | The version in force, or the built-in one |
| `evaluate_approval_matrix` | `/evaluate-approval-matrix` | `application-read` | Decides a described enrollment with the given rules; stores nothing |
| `save_approval_matrix` | `/save-approval-matrix` | `approval-matrix-write` | Validates, saves the next version and posts `ApprovalMatrixUpdated(event, version, sha256)` to the electoral log in the same transaction |

## Decision record

`verify_application` stores the decision in the application's `annotations.decision`:

```json
{
  "matrix_version": 1,
  "matrix_source": "BUILT_IN",
  "rule": 5,
  "conditions": {"differing": "exactly_1", "fields": {"embassy": "MATCHES"}},
  "decision": "PENDING",
  "reason": "NO_VOTER",
  "inputs": {
    "identity": null,
    "voter_found": true,
    "already_enrolled": false,
    "valid_id": "philippinePassport",
    "fields": {"firstName": "DIFFERS", "middleName": "MATCHES", "lastName": "MATCHES", "dateOfBirth": "MATCHES", "embassy": "MATCHES"},
    "differing": 1
  },
  "candidates": 1,
  "accepted_candidates": 0
}
```

`rule` is empty when Otherwise decided. It contains no data beyond what the application already
holds, and follows the application's retention.

## Admin portal example panel

The matrix screen (`packages/admin-portal/src/resources/Approvals/ApprovalMatrix.tsx`) decides one
**applied example** with `evaluate_approval_matrix` and the rules on screen. The rule it highlights
is derived from that query in the same render: while a request for the current rules and example
is on its way, failed, or answered with matrix errors, no rule is highlighted. With
`fetchPolicy: "no-cache"`, Apollo Client 3.8 drops the result of earlier variables, so a late
answer can't highlight a rule of an earlier rule order.

The editor keeps a draft; only **Apply** replaces the applied example. **Fill in the recorded
details** / **Fill in the sample** only reset the draft (and are `aria-disabled` while the draft
already holds them). Cancel and Apply stay on one row: in narrow editors (a container query on the
editor's width) the footer's text and padding become compact rather than wrap. On a phone the
editor reaches into its card's padding, and below 368 px (`SMALL_PHONE` in `ApprovalMatrix.tsx`)
the page and its cards give up their padding, so on windows of 320 px and wider, beside the
collapsed side menu, the editor is never narrower than 174 px. The panel's title follows where the
matrix was opened from: **Try an example** from the toolbar, **Simulating the enrollment of …**
from an enrollment.
The heading over the details in use, and whether the unreadable notice shows, follow `panelState`
(`approvalExample.ts`, `EPanelState`): the sample or the administrator's own example; the
enrollment's recorded details or other details; or, when its decision can't be read, the sample
or other details started from it. Other details are counted on what the rules see, as the
comparison counts them.

The card is sticky on wide screens and sized from measurements (`matrixLayout.ts`): the page
header's height, the save bar's height, and the content below the rules in the layout's scroll
frame, so it stays between the header and the save bar wherever the page is scrolled.

Opened from an application, the example is read from its decision record
(`approvalExample.ts`, `recordedDecision`). The reading is strict:

- A matrix version that isn't a whole number from 1, an unknown status, a rule that isn't a
  position from 1, an unknown invariant, an unknown field result, malformed inputs, or inputs
  that disagree with the number of compared candidates (a voter exactly when candidates were
  compared) make the record unreadable: the panel says so and the sample is used instead.
- A candidate count (`candidates`, `accepted_candidates`) that isn't a whole number from 0 is
  unknown, not unreadable: the recorded inputs are still replayed, but with an unknown number of
  compared candidates the decisions aren't compared. A
compared field without a recorded result is sent without one (as the backend decides it) and
shown as not recorded; the middle name of an ID that gives first and middle name together
(`GivenNames::TOGETHER`) has no result of its own. The decisions are compared only when the
applied example replays the record exactly and one candidate or none was compared.

The panel's source is in-session state of `EditElectionEventApprovals`, scoped to the tenant and
event. The tab selection isn't written to the address, so a reload loses it.

## Bundles and the janitor

The election event bundle has an optional `approval_matrix`
(`sequent-core/src/election_config/schema.rs`). The export writes the matrix in force; the import
validates it and saves it as version 1; the workbook builder carries it over from a base export.

The janitor adds `templates/COMELEC/approvalMatrix.json` to the bundle. `--approval-matrix <path>`
chooses another preset, such as `templates/association/approvalMatrix.json`.

## Tests

```bash
cd packages
cargo test -p windmill --lib approval_matrix
cargo test -p windmill --lib matrix_tests
cargo test -p windmill --test postgres_approval_matrix
cargo test -p harvest approval_matrix
cargo test -p electoral-log approval_matrix
python3 -m unittest scripts.dev.test_janitor_approval_matrix_preset   # from the repository root
```

The database tests need the PostgreSQL server that `HASURA_DB__*` names.

## Open decisions

- **Annex B.** COMELEC's approved validation matrix is entered with the editor as a new version;
  until then version 1 applies.
- **Who may change the matrix.** The `admin` group has `approval-matrix-write` in the tenant realm
  templates; other roles are granted it in Keycloak. A version applies as soon as it is saved.
- **When changes may apply.** The matrix can change while enrollment is open.
