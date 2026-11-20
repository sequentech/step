---
id: configuration_signing_trust
title: Configuration Package Signing Trust
sidebar_label: Configuration package trust
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

A tenant's `settings.configuration_signing` decides which configuration
packages it imports. Election Architect signs them; this page covers the
platform's side.

## The setting

```json
{
  "policy": "required",
  "package_roots": "-----BEGIN CERTIFICATE-----\n…",
  "staff_roots": "-----BEGIN CERTIFICATE-----\n…",
  "required_approvals": 2
}
```

| Field | Default | Meaning |
| --- | --- | --- |
| `policy` | `optional` | `ConfigurationSigningPolicy`. `required`: only a signed package that passes every check imports. `optional`: a signed package is checked, an unsigned file imports as before. |
| `package_roots` | empty | PEM certificates the package signing key must chain to. |
| `staff_roots` | empty | PEM certificates approvers' certificates must chain to. |
| `required_approvals` | 0 | Valid approvals from distinct people. Format 1: distinct by certificate subject or fingerprint. Format 2: distinct by **enrolled signer ID** (below). |
| `minimum_package_format` | absent | `v1` or `v2`. Absent only on a legacy record, read as `v1`. With `v2`, only format-2 packages import, and a format-1 package is refused as a downgrade. |
| `final_signer_separation` | `separate` | Format 2. `separate`: the final signer must not be one of the package's approvers. `allowed`: they may be. |
| `signers` | empty | Format 2. Enrolled signers: `{id, name, roles: ["approver" \| "final-signer"], keys: [{spki_sha256, label}]}`. |

A tenant without the setting behaves as `optional` with no roots: unsigned
files import, signed packages are refused as untrusted. An explicit `null`
field is unreadable, and every import fails until the setting is saved again.

## Format 2: enrolled signers

In format 1, a person is identified by a certificate's subject or fingerprint.
Format 2 identifies them by an **enrolled signer ID** that the tenant's policy
lists, with the roles it may sign in and the keys it uses.

- **Stable identity.** A key is enrolled by its SPKI pin, the SHA-256 of its
  SubjectPublicKeyInfo DER. A reissued certificate for the same key keeps the
  pin, and one person may enroll several keys. Approvals count **distinct
  signer IDs**. Two approvals by one ID with two keys are refused as a
  duplicate person. Issuer and serial alone never identify anyone, and a key
  belongs to exactly one ID.
- **Binding.** The approval payload is
  `sequent.configuration-approval/2` canonical JSON. It names the tenant,
  `external_id`, revision, `content_sha256`, `policy_sha256`, the signer ID
  and the role `approver`. The format-2 manifest's `signing` object binds the
  final signature to the same tenant and policy digest, and to the final
  signer's ID.
- **`policy_sha256`** (`signing_policy::policy_sha256`) is the SHA-256 of a
  canonical JSON of:
  - the policy, the approval count, the effective minimum format and the
    separation;
  - both root sets, as sorted SHA-256 digests of each certificate's DER, so
    PEM order and whitespace don't matter;
  - the signers, sorted by ID, with roles and keys as sets. Key labels are
    excluded.

  Any other change to the policy changes the digest.
- **Algorithms.** ECDSA P-256 with SHA-256, or RSA-PSS exactly (SHA-256,
  MGF1-SHA-256, 32-byte salt, 2048–4096-bit modulus). PKCS#1 v1.5 is refused in
  format 2, and accepted in format 1 only.

`check_policy` refuses a policy that can't be met:
`minimum-format-required` (signers or separation without a minimum format),
`optional-with-v2`, `no-signers`, `no-final-signer`, `too-few-approvers`,
`separation-unsatisfiable`, `signer-id`, `duplicate-signer`, `signer-name`,
`signer-roles`, `signer-keys`, `key-pin`, `duplicate-key` and `shared-key`,
all under `configuration-signing.`.

`check_stored` runs on every **stored** record that has any format-2 field,
before admission. A malformed record is never read with legacy defaults. Every
file, signed or not, is refused with
`configuration-signing.invalid-stored-policy` until an administrator saves a
valid policy. A record with no format-2 field is legacy, and is read exactly as
before.

The installer's file `signing-policy.json` is exactly `{tenant_id, signing}`.
The Admin Portal loads it only for that tenant and refuses unknown fields.
The standalone Election Architect serves the same file as
`trust/signing-policy.json`. A package never carries or changes trust.

## Setting it

The only writer is `set_configuration_signing` (Hasura action, Harvest
`POST /set-configuration-signing`), used by **Settings → Configuration package
→ Package signing trust**. It needs all six permissions a package application
needs, on that tenant. The setter:

1. Reads the input strictly: an unknown field or policy is refused, not dropped.
2. Runs `ConfigurationSigning::check` (sequent-core `package_verify`). Errors are
   returned and nothing is written:
   - `configuration-signing.no-package-roots`: required with no package roots;
   - `configuration-signing.no-staff-roots`: approvals with no staff roots;
   - `configuration-signing.too-many-approvals`: more than 16;
   - `configuration-signing.not-a-ca`, `root-expired`, `root-not-yet-valid`:
     the root itself is unusable now;
   - `configuration-signing.not-only-certificates`: any PEM block other than
     `CERTIFICATE`, or text outside the blocks. A pasted private key is refused,
     never stored or audited.

   `configuration-signing.no-approvals` (required, zero approvals) is a warning.
3. Stores the canonical certificates only (`ConfigurationSigning::canonical`).
4. Try-locks the tenant's package-application claim and its import lock. If a
   package application or an election event import (signed or not) is running,
   it answers `saved: false` with the error `configuration-signing.busy` and writes nothing; it does not wait. The administrator saves again once that run has finished. While the setter's
   transaction is open, neither can start, so trust never changes under one.
5. Writes the key with `postgres::tenant::write_configuration_signing` and stages
   an audit entry with the old and new values and the trusted fingerprints.

## What keeps it

- **Database guard** (migration `configuration_signing_guard`): a trigger on
  `sequent_backend.tenant` refuses any INSERT, UPDATE or DELETE that would add,
  change, null or remove `settings.configuration_signing`. This includes
  GraphQL `_append`/`_delete_key`, rewriting the whole settings object,
  changing the row's id, and deleting and recreating the tenant. Writes that leave
  the key as it is pass. The exception is a transaction-local GUC,
  `sequent.configuration_signing_tenant`, naming the one tenant. It is set by
  `allow_configuration_signing_change`, which only the setter and super-admin
  tenant deletion (`TENANT_DELETE`) call. It ends with the transaction and does
  not stay on pooled connections.
- **Packages**: `configuration_signing` is a reserved setting
  (`RESERVED_SETTINGS`). A package that would change it is blocked in the
  preview, and the merge keeps the installed value.
- **Backups**: a tenant restore keeps the installed value, and also keeps its
  absence (`keep_reserved_settings`), so old backups still restore.

## Admission

Every package preview and application, and every election event import, admits
the file with `admit_for_tenant_at` against the tenant's setting at that
moment, before anything is read or changed. For format 2, admission also
checks the following against that tenant's **current** policy and the
platform's **own trusted clock** (never a time the package claims):

- the package's tenant;
- the policy digest, so a policy changed after approval, including between
  preview and apply, is refused;
- enrollment and roles, and separation;
- the format: a downgrade, a mixed-format package or an unsigned file is
  refused.

The locked revision check is unchanged: a revision that isn't newer than the
last imported one is refused as a rollback (`package.rollback`).

**Replay of the same package.** A package application
(`configuration_package_sections::admission_problem`, on every section
preview) refuses a **format-2** package that is already imported. The refusal
reads "… is already imported (into event …); a format-2 package is imported
exactly once", and is given at the initial preview of every run, before any
write or reuse of a prior record. There is one admission:
`PlatformTarget::event_imported_by_this_run`. It is set only when **this
invocation's** election-event submission succeeded (its import returned Ok
and its task row is SUCCESS), and it lets that same run's convergence
re-preview pass. A matching derived event ID or a prior record never admits
it.

A run that stopped after the event import committed has nothing left to
apply, because the event is the last section. Its retry is therefore refused
and writes nothing, and the operator inspects the imported state and the task
instead. **Format 1** keeps its same-event resume. The direct election-event
importer (`admit_document`) refuses any already-imported package under the
tenant import lock.

**Revocation:**

- Removing a key from the policy withdraws it at once.
- Configured CRLs are honoured.
- A missing or stale CRL is **not** treated as revoked, and no revocation
  freshness is claimed. A refused package changes nothing. Election event
imports hold the tenant's import lock from before the policy is read until the
import is recorded, signed or not.

## Tests

From `packages/`, with a disposable PostgreSQL named by `HASURA_DB__*`:

```sh
cargo test -p sequent-core --features default_features,election_config_signing,election_config_xlsx --lib election_config
cargo test -p windmill --lib -- configuration import_tenant
cargo test -p windmill --test postgres_configuration_signing --test postgres_configuration_packages
yarn --cwd admin-portal jest src/resources/Settings/configurationSigning.test.ts src/resources/Settings/ConfigurationSigningSettings.test.tsx
```


## Isolated browser-security-key preview

This is default-off, non-production work. It does not replace certificate
format 1/2 or enable signing in an enrollment-only or pending-only installation.
No hardware-key, production-deployment or end-to-end election application claim
follows from software-key tests.

Architect's preview freezes the complete manifest before issuing the final
WebAuthn challenge. Its final seal covers the exact manifest bytes, tenant,
relying party, revision, previous manifest link and frozen ordered approval
digests. The separate final signer cannot have approved. A successful finish
assembles the stored delivery, verifies it with Step's core and atomically
records the package and signed revision. The ZIP carries
`manifest.webauthn.json`, not a certificate signature. A prior certificate
package is not silently made the predecessor of a browser chain.

Windmill must be provisioned **separately**, never from the ZIP or an imported
tenant setting:

- `WINDMILL_WEBAUTHN_PACKAGES_PREVIEW=nonproduction-isolated-test`
- `WINDMILL_WEBAUTHN_PACKAGES_TENANT_ID`: destination tenant v4 UUID
- `WINDMILL_WEBAUTHN_PACKAGES_ROSTER_FILE`: operator-provisioned public roster
- `WINDMILL_WEBAUTHN_PACKAGES_ROSTER_SHA256`: independently checked lowercase SHA-256
- `WINDMILL_WEBAUTHN_PACKAGES_REQUIRED_APPROVALS`: positive threshold

The roster names the canonical tenant v4 UUID. UUID aliases cannot select another
policy. An invalid preview switch fails closed. For the configured tenant,
unsigned and certificate packages are refused, including during later
application rechecks. ZIP directory ambiguity and duplicate/aliased signature
metadata are rejected before unsigned fallback. Planning uses verified members.

Under the tenant import lock, the next package must name the last persisted
revision and manifest digest exactly; identical replay and older packages are
refused. The first visible signed revision may exceed 1 after withdrawn drafts,
but it must have no previous link when this tenant has no import history.
Whole-package application requires an applicable election-event section so the
event importer records the chain anchor. Sections remain separately committed:
an interruption can leave earlier sections applied, not an atomic whole-package
rollback. Re-preview and the persisted chain govern retries.

Public roster export is not automatic receiving trust. Operators independently
verify, provision and rotate both roster and pin. Changed model/FIDO trust,
pending-only mode and revoked keys prevent new producer use. Stored browser
artifacts require their immutable tenant/relying-party owner and a preview build;
old detached preview seals cannot acquire ownership through a GET request.

The paired integration fixture is emitted only by the synthetic Architect test
when `ARCHITECT_SYNTHETIC_PACKAGE_FIXTURE_DIR` is set. With that directory,
independent Windmill policy and a disposable migrated PostgreSQL:

```sh
cargo test -p windmill --test postgres_configuration_packages   browser_producer_package_uses_real_admission_and_persisted_chain -- --ignored
```

That test exercises real `admit_document`, `record`, committed history, replay
refusal and the next exact previous link. It does not apply an election through
live Hasura or Keycloak.
