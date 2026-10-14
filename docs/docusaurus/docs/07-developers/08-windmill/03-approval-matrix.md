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
`ALREADY_APPROVED`, `INSUFFICIENT_INFORMATION`, `IDENTITY_NOT_VERIFIED` or `OTHER`.

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
