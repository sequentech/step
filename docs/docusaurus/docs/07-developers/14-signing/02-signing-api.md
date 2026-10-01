---
id: signing_api
title: Signing API Contract
sidebar_label: API contract
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The Admin Portal reaches the signing routes through Hasura actions, which forward their
input to Harvest and its JSON answer back. Every action takes `election_event_id` or a
`request_id`; the tenant comes from the token. This page is the contract; the portal
mirrors it in `packages/admin-portal/src/lib/signing/api.ts` (the widget),
`src/queries/SigningSettings.ts` (the Signatures tab) and `src/lib/signing/types.ts`, and
the Rust side in `sequent_core::signing` and the Harvest route types. A change starts here
and is mirrored in all of them.

## Errors

Every signing route answers errors as
`{"message": "…", "extensions": {"code": "<code>", "check": "<check id>"?, "status": "<request status>"?, "reason": "…"?}}`.
Hasura forwards `message` and `extensions`; the portal classifies on `extensions.code` and
falls back to the HTTP status.

| Code | HTTP | When |
|---|---|---|
| `signing-refused` | 422 | A certificate or signature check failed; `check` is its id. |
| `already-signed` | 422 | This user, certificate, key or holder already signed the request. |
| `request-closed` | 409 | The request isn't waiting (`status` says what it is) or it expired. |
| `stale-revision` | 409 | PDF: the prepared revision no longer extends the latest signed one. The only case that makes the client prepare again. |
| `conflict` | 409 | A stale `expected_revision` (rules, checks), a certificate already registered, or another certificate preparing the same PDF revision. |
| `locked-down` | 409 | A rule edit on a locked-down election event. |
| `signing-required` | 409 | The change needs signatures: start its signing request. A transmission send answers it, with `status`, while the Transmit results rule needs signatures and the package's request hasn't run. |
| `signatures-short` | 409 | A transmission package sent without a signing request has fewer uploaded signatures than its threshold. |
| `forbidden` | 403 | A missing permission, a Post outside the user's labels, or a requester the rule doesn't allow. |
| `not-found` | 404 | An unknown request, rule, issuer, certificate, user or Post. |
| `invalid` | 400 | Bad input, such as more signatures than any Post can give. `reason` is `document` (no document to sign, or a signature of the wrong kind) or `document-fields` (the PDF's empty fields don't match the request's signatures). |
| `unauthorized`, `internal` | 401, 500 | As usual. |

Check ids, in display order: `trusted-issuer`, `valid-now`, `signing-key-usage`,
`not-revoked`, `registered`, `registered-to-other`, `already-signed`, `post-binding`,
`signature`.

## Widget actions

| Action | Route | Input | Output | Permission |
|---|---|---|---|---|
| `signingGetRequest` | `POST /signing-requests/get` | `request_id` | `{panel}` (below) | The requester, `sign-<action>` or `signing-requests-read` |
| `signingCheckCertificate` | `POST /signing-requests/<id>/check-certificate` | `request_id`, `chain_pem[]` (leaf first) | `{checks: [{id, ok, detail}], certificate, registration, revocation_status}` | `sign-<action>` |
| `signingPdfPrepare` | `POST /signing-requests/pdf-prepare` | `request_id`, `chain_pem[]` | `{revision, digest_b64, signing_time}` | `sign-<action>` |
| `signingApprove` | `POST /signing-requests/approve` | `request_id`, `chain_pem[]`, `algorithm`, `payload_signature_b64`, `document_signature_b64?`, `pdf_cms_b64?`, `revision?` | `{status, count, required}` | `sign-<action>` |
| `signingOpenFailure` | `POST /signing-requests/open-failures` | `request_id`, `file_name`, `reason` (`wrong-password`, `unreadable`, `no-key`) | `{request_id}` | `sign-<action>` |
| `signingHandover` | `POST /signing-requests/handover` | `request_id` | `{request_id}` | `sign-<action>` or the requester |
| `signingCancel` | `POST /signing-requests/cancel` | `request_id`, `reason?` | `{request_id}` | The requester or `signing-requests-cancel` |

`detail` per check: `trusted-issuer`, the anchor's name; `not-revoked`, the last good list
download (RFC 3339); `registered`, the registration date, or null on first use;
`registered-to-other`, the other person's display name.

`signingPdfPrepare` gives the same answer again for the same signer, certificate and latest
revision; another certificate on the same revision within a few seconds is a `conflict`.
Its refusals are logged like the approve step's.

### The panel

`signingGetRequest` answers:

- the request: `id`, `action`, `status`, `code`, `required`, `count`, `expires_at`,
  `election_id`, `area_id`, `trustee_id`, `subject`, `canonical_payload` (the exact bytes as
  a string), `payload_sha256`, `document_sha256`, `requested_by`, `requested_by_name`,
  `cancel_reason`;
- the rule snapshot;
- `signers`: `[{user_id, username, display_name, title, is_you, status: signed | not-signed, signed_at?, certificate_cn?}]`;
- `document_url`: a short-lived link to the request's own document, the one whose hash the
  payload names (a PDF's base revision with its empty fields, or a transmission's EML). The
  widget downloads it and checks the hash;
- `document_revision` (PDF only): `{revision, sha256, signed_count, url}`, the document as it
  stands, for viewing only;
- optional `election_name`, `area_name`, `document_name`, `details: [{key, label?, value}]`
  and `time_zone`.

The widget shows values from the parsed canonical payload and refuses mismatches, so the
backend must satisfy:

- `details[].key` is a field of the payload's subject, and `value` its display form
  (strings as they are, numbers as text, arrays joined with ", ");
- a PDF action's subject carries `document_sha256`, and a transmission's `eml_sha256`, equal
  to `request.document_sha256`;
- the payload is canonical with domain `step-signing/v1`, and its request id, code, action,
  tenant, event, `election_id` and `area_id` equal the request's.

## Settings actions

| Action | Route | Input | Output | Permission |
|---|---|---|---|---|
| `signingPutRule` | `POST /signing-rules/put` | `action`, `requirement`, `signatures`, `requester_signing`, `expires_minutes` (null = no limit), `expected_revision`, `roles?: {add: [group id], remove: [group id]}` | `{revision, cancelled: [uuid], rule, short_posts}` | `signing-rules-write`; with `roles`, also `role-read` and `role-write` |
| `signingRuleCapacity` | `POST /signing-rules/capacity` | `action`, `signatures?` | `{max, posts, posts_short, roles: [{id, name, path}], waiting, config_version}` (`posts` entries are `{election_id, name, count}`) | `signing-rules-read` |
| `signingExportRequests` | `POST /signing-requests/export` | `filters?` | `{document_id, sha256, rows}` (a CSV document, downloaded like other documents) | `signing-requests-export` |
| `signingImportIssuers` | `POST /signing-issuers` | `pem?` or `der_base64?` | `{imported, skipped, errors}` | `signing-issuers-write` |
| `signingDeleteIssuer` | `DELETE /signing-issuers/<id>` | `issuer_id` | `{issuer_id}` | `signing-issuers-write` |
| `signingPutChecks` | `PUT /signing-checks` | `revocation_check`, `crl_unavailable`, `registration`, `post_binding`, `expected_revision` | `{revision}` | `signing-checks-write` |
| `signingRegisterCertificate` | `POST /staff-certificates` | `user_id`, `election_id?`, `pem`, `linked_to?` | `{certificate_id}`; a `registered-to-other` refusal adds `extensions.user_id` and `display_name`, so the UI can offer the link | `signing-certificates-register` |
| `signingRevokeCertificate` | `POST /staff-certificates/<id>/revoke` | `certificate_id`, `reason` | `{certificate_id}` | `signing-certificates-revoke` |

`GET /staff-certificates/mine` lists the caller's own registrations. The tab reads the
tables through Hasura selects with `x-hasura-role` set to the sub-tab's read permission;
the tables carry display names written at write time (`updated_by_name`,
`requested_by_name`, `display_name`, `user_display_name`, `registered_by_name`,
`revoked_by_name`).

## Existing routes

Routes that start a protected action answer their usual output plus an optional
`signing_request: {id, code, required, expires_at}` when the rule is `Required`; their
Hasura output types gain the optional field. The trustee routes (`/check-private-key`,
`/restore-private-key`) take an optional `key_share_sha256` and `signing_request_id`. A
transmission package is sent only when its request is executed.

## Enums

Values are kebab-case strings in JSON and in the database.

| Enum | Values |
|---|---|
| Action | `initialize-voting`, `open-voting`, `close-voting`, `generate-election-returns`, `generate-reports`, `transmit-results`, `approve-voter`, `approve-configuration`, `key-ceremony`, `tally-key` |
| Requirement | `not-required`, `required` |
| Requester signing | `allowed`, `not-allowed` |
| Request status | `waiting`, `completed`, `executed`, `cancelled`, `expired`, `failed` |
| Cancel reason | `by-requester`, `by-operator`, `rule-changed`, `payload-changed`, `superseded`, `certificate-revoked` |
| Revocation check | `check`, `dont-check` |
| List unavailable | `refuse`, `accept-unchecked` |
| Registration | `on-first-use`, `security-officer-only` |
| Post binding | `one-post`, `any-post` |
| Signature algorithm | `rsa-pkcs1-sha256`, `ecdsa-p256-sha256` |
| Authority purpose | `voter-sign-in`, `staff-signatures` |

## Canonical payload and signing code

The payload every signer signs is UTF-8 JSON with keys sorted recursively by bytes, no
whitespace and no floats. Sort explicitly: Windmill enables serde_json's
`preserve_order`, so a `Map` keeps insertion order there. Common fields:

```json
{"action":"…","area_id":null,"code":"7F3A-91C2","config_revision":"…","created_at":"2026-01-01T10:00:00Z",
 "domain":"step-signing/v1","election_event_id":"…","election_id":"…","expires_at":"…",
 "request_id":"…","requested_by":"…","rule_revision":3,"subject":{…},"tenant_id":"…"}
```

Subjects per action:

| Action | Subject |
|---|---|
| Initialize voting | `{publication_id}` |
| Open voting / Close voting | `{channel}` / `{channels}` (sorted) |
| Election returns, other reports | `{report_type, document_sha256, template_id}` |
| Transmit results | `{tally_session_id, package_sha256, eml_sha256, destinations}` |
| Approve a voter | `{application_id, applicant_registry_id, decision: "approve", submitted_at, reason, registry_record}` |
| Approve a configuration version | `{ballot_publication_id, digest, signing_rules, scheduled_events, ballots_and_contests}` |
| Key ceremony / tally key share | `{keys_ceremony_id or tally_session_id, trustee_id, key_share_sha256, ceremony_name, trustee_name}` |

Everything the dialog shows is in the subject, so what is shown is what is signed.

- `payload_sha256` = SHA-256 of the canonical payload, lowercase hex.
- Signing code = the first 40 bits of
  `SHA-256("step-signing-code" ‖ request_id ‖ SHA-256(canonical(subject)))` in Crockford
  base32, formatted `XXXX-XXXX`. It doesn't depend on the payload, so the payload can carry
  it.
- `payload_signature`: RSASSA-PKCS1-v1_5 or ECDSA P-256 (DER) with SHA-256 over the
  canonical payload bytes. PDF actions add `pdf_cms`; a transmission adds
  `document_signature` over the EML bytes, which the results package carries per signer
  with the signer's public key.
