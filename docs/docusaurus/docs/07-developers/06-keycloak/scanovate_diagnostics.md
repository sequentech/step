<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Scanovate failure diagnostics

Failed identity verification is recorded in the election event's **Logs** tab as
`REGISTER_ERROR / scanovate_verification_failed`. Open the entry to see the
`scanovate_*` fields in **Sign-in and account security**. The existing
`/electoral-log` route requires `logs-read`, checks the caller's tenant and
applies the event/election scope. Export uses existing log export permissions.
This change grants no new roles or access.

The error page's reference is `scanovate_attempt_id`, an independent random UUID,
not a Keycloak session ID or verification token. A retry starts a new attempt.
`scanovate_case_id` links to the server-created Scanovate case; it is absent when
configuration fails before a case exists. The signed log envelope supplies
tenant and election event. `scanovate_occurred_at` records Keycloak's occurrence
time in UTC; the board timestamp records when the durable entry was built. An unknown actor is expected
before an account is matched or created.

## Field contract

All fields below have the `scanovate_` prefix. The producer emits a fixed
allowlist of scalar strings, at most 80 characters each. Unknown fields and
inherited form/session details are removed from a cloned failure event. Success
and pass-through event metadata remains intact. No raw
request, exception text, response or arbitrary configurable extraction rule is
serialized into diagnostics.

| Field | Meaning |
| --- | --- |
| `attempt_id`, `case_id` | Independent attempt UUID and optional verification case UUID |
| `occurred_at` | UTC occurrence time supplied by the Keycloak listener, independent of queue delays |
| `service_error`, `http_status` | Bounded category (`http_error`, `timeout`, `connection`, `tls`, `transport`, `malformed_response`, `provider_response`, `interrupted`) and optional numeric HTTP status 100–599; no error text or response body |
| `config_component` | Invalid configuration group: `capture`, `liveness`, `face_match`, `ocr`, `validation_rules` or `storage_rules`; never setting values |
| `stage` | `configuration`, `capture`, `liveness`, `face_document`, `face_holding`, `ocr` or `validation` |
| `error` | Allowlisted error-page category, including retry exhaustion |
| `failure_reason` | Underlying category, retained when `error` becomes `scanovateMaxRetriesError`; custom rule messages map to `scanovateAttributesError` |
| `can_retry`, `failed_attempts` | Retry availability and existing counted-failure value, capped at 9999 in diagnostics only; service/configuration failures need not increment it |
| `entered_document_type`, `entered_embassy`, `entered_country` | Selected realm profile options only; missing values say `missing`, unconfigured/free-text/oversized values say `unrecognized_redacted`. Country is the entered choice, never network geolocation |
| `entered_email_state`, `entered_phone_state` | `missing`, `present_redacted` or `unknown` when an original snapshot is unavailable; **no address or number is retained**. Phone refers to the existing mobile-number profile attribute, not every possible messaging channel |
| `entered_document_number_state`, `extracted_document_number_state` | Presence only, never a document number, MRZ or hash of one |
| `liveness_status`, `liveness_passed` | Allowlisted callback status and verdict; `unknown` when no authenticated final result is available, which does not establish why a callback is missing |
| `face_match_document`, `face_match_holding` | Existing similarity scores to four decimals; non-finite/out-of-range values become `invalid` in diagnostics only |
| `ocr_front_status`, `ocr_back_status`, `ocr_front_result`, `ocr_back_result` | Allowlisted response/processing categories per attempted side; unknown vendor text becomes `unknown` |
| `extracted_document_type` | Allowlisted document/MRZ type; other values are redacted |
| `extracted_issuing_country`, `extracted_nationality` | Three-uppercase-letter codes only; other values are redacted |
| `check_template_matching_valid`, `check_document_in_frame_valid`, `check_expiry_date_valid`, `check_face_size_valid`, `check_barcode_valid` | Boolean scan checks, or `missing`/`invalid` when absent/malformed |

Extracted fields/checks are available only after OCR produces combined readable
results. Earlier failures have no invented reading. Per-side status is retained
when a later OCR side fails. Entered fields are captured once before OCR may replace authentication notes,
and that sanitized snapshot survives scan retries. A newly accepted registration
form resets it. A confirmation created before the snapshot existed reports
unknown/redacted entered data. The snapshot includes form defaults as well as
entered values; the document-number presence marker does not prove it was typed. The configured document-type note is respected; embassy,
country, email, mobile and document-number fields use the established profile
names. No other profile fields are copied.

The producer never adds access/session/verification tokens, callback secrets,
URLs, document images, face images, biometric templates, names, dates of birth,
full passport/national ID numbers or raw MRZ text. Ordinary application logs use
fixed messages, normalized status categories and case UUIDs; vendor response
excerpts and transport/parser exception chains are not logged. A matched roster
user retains the existing server-resolved user ID attribution even before the
authenticated context user is set.

The listener applies the same shared allowlist again and serializes the diagnostic
object into both the existing task body and details. The election-board writer
consumes the body; the tenant fallback consumes details. The general secret-name
filter remains unchanged: the barcode check survives only within this bounded
Scanovate failure object. Other event serialization is unchanged.

## Full contact lookup remains a design gap

This envelope does **not** let support identify a person by entered email or
phone. The current electoral log is signed durable audit evidence, with general
log-reader/export access, and enrollment event helpers deliberately exclude
contact values. A separate restricted support record with defined retention,
deletion and read auditing is needed before retaining those values for failed
attempts. Its write path should snapshot entered contacts before OCR, separate
them from extracted data, and reference the same attempt, case, tenant and event.
It must not put contacts in the signed log or ordinary queue as a workaround.
No such record, permission or retention policy is introduced here. Never infer
that a redacted field is absent.

## Approvals and the ID field

Scanovate runs before `LookupAndUpdateUser` calls `/verify-application`. A failure
at this earlier stage emits an event but creates no approval application. The
Approvals **Rejected** view contains persisted applications rejected by rules or
a reviewer. A failed scan is not a rejection decision; these states are unchanged.

The COMELEC profile template marks the ID-number input with
`html-attribute:style: visibility:hidden` and a placeholder default. Its edit
permissions include the user; the `sequent.read-only` name alone is not the
mechanism. Scanovate's configured store rule fills that attribute from OCR's
`document_number`. A visible label without an editable control can therefore be
a configuration/rendering issue, rather than a missing value to type. Check the
actual event profile and rendered form before changing it. Callback authorization
is a separate secret/token check; this field does not explain a callback HTTP 401.

## Synthetic validation

A shared synthetic `scanovate-failure-contract.json` fixture is checked by the
authenticator, actual custom listener/queue publisher, Rust queue parser and signed
election-board builder, and the admin log-entry parser.

The `scanovate-authenticator` suite covers redaction, missing fields, bounded
status/error categories, malformed callback bodies, callback authorization,
retry isolation and preserved underlying failure alongside the existing flow.
Run through the Keycloak reactor:

```sh
cd packages/keycloak-extensions
mvn -pl scanovate-authenticator -am test
```

The pure field contract also runs without Maven or downloaded dependencies:

```sh
module=packages/keycloak-extensions/scanovate-authenticator
out=$(mktemp -d)
javac --release 17 -d "$out" \
  "packages/keycloak-extensions/message-otp-authenticator/src/main/java/sequent/keycloak/authenticator/diagnostics/ScanovateDiagnostics.java" \
  "$module/src/test/java/sequent/keycloak/scanovate_authenticator/DiagnosticContract.java"
java -cp "$out" sequent.keycloak.scanovate_authenticator.DiagnosticContract
```

This narrow contract does not replace reactor tests or an authorized staging
enrollment. Neither test path calls a live provider or uses real voter data.
