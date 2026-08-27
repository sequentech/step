# Encrypted voter attributes for voter-level outputs

## Part 1 — Feature description

Election-event user-profile attributes can be marked as **secret voter attributes**. Their values are encrypted by Step before being stored in Keycloak and are never returned by the normal voter list, voter detail, filter, export, or Keycloak-facing paths.

Authorized administrators can see whether a secret value is set, explicitly reveal it, replace it, or clear it in the Admin Portal. Revealing and changing secrets are separate capabilities:

- `voter-secret-attribute-read` permits an explicit reveal and the use of a secret in an authorized voter-level output.
- `voter-secret-attribute-write` permits creating, replacing, clearing, and importing secret values.
- The corresponding ordinary permission is still required. For example, revealing requires both `voter-read` and `voter-secret-attribute-read`; editing requires both `voter-write` and `voter-secret-attribute-write`.

Voter-level communication and report templates can declare the secret attributes they need. At execution time, the worker decrypts only those attributes and injects them into the existing voter variable structure. A secret attribute named `customerReference`, for example, remains available as `{{user.customerReference}}` and `{{lookup user.attributes "customerReference"}}`. Aggregate reports never receive voter secrets.

The first version is deliberately limited to custom, report-only voter attributes. Built-in identity and operational fields such as username, email, name, mobile number, area, date of birth used for authentication/reconciliation, authorization, vote weight, and voting status cannot be marked secret. A secret attribute also cannot be used by Keycloak authentication flows, registration, token mappers, uniqueness checks, filters, sorting, or voter self-service because Keycloak sees only ciphertext.

### User-visible behavior

| Area | Behavior |
|---|---|
| User-profile configuration | Add `"sequent.secret": "true"` to an eligible custom attribute's annotations. In v1 it must not use Keycloak `required` semantics or value validators, because Keycloak stores and validates the ciphertext envelope rather than the submitted plaintext. |
| Voter list | Do not return the value, ciphertext, a filter, a sort option, or a data column. It may show only a generic “secret set” indicator if useful. |
| Voter create/edit | Show a masked secret input to users with write permission. Omitted means preserve, **Replace** sets a new value, and **Clear** removes it. |
| Voter detail | Show only “value set” until a user with read permission chooses **Reveal**. Do not cache a revealed value. |
| Import | Accept plaintext in a configured secret column only when the importer has secret-write permission; encrypt before inserting into `user_attribute`. |
| Normal export | Omit secret columns. |
| Sensitive export | An explicit option includes decrypted secret columns only with `voter-secret-attribute-read`; the resulting private document retains that restriction when downloaded. |
| Voter-level output | Decrypt only the secret names declared by the template/report and only for an authorized execution. Preserve the existing `user.*` and `user.attributes.*` variable shapes. |
| Logs and audit | Never log ciphertext, plaintext, rendered bodies, or request bodies containing secrets. Audit who revealed, changed, cleared, imported, or used which attribute, without recording its value. |

### Acceptance criteria

- Keycloak's `user_attribute.value` contains a recognizable authenticated-encryption envelope and never the plaintext for an active secret attribute.
- A normal `voter-read`, `voter-write`, `voter-export`, or `notification-send` user cannot obtain or overwrite a secret value without the additional secret permission.
- Generic APIs do not expose either plaintext or ciphertext, including during list, detail, error, task, and export flows.
- A standard export omits secret columns; an explicitly requested decrypted export requires secret-read permission both when generated and downloaded.
- Admin reveal, set, preserve, and clear semantics work for single- and multi-valued attributes and are audited without values.
- Email, SMS, and supported per-voter report templates receive only their declared decrypted attributes; outputs without a declaration receive none.
- Existing realms and templates behave exactly as before when no attribute has `sequent.secret=true`.
- Tampered, unknown-version, wrong-voter, wrong-event, or legacy plaintext values fail closed and are observable without being included in error messages.

## Part 2 — Implementation plan

### 1. Current Step paths that the change must cover

| Concern | Current implementation | Consequence for this feature |
|---|---|---|
| Master secret and encryption | `packages/windmill/src/services/vault/vault.rs` loads `master_secret` from AWS Secrets Manager, HashiCorp Vault, or `MASTER_SECRET`, then uses `strand::symm` authenticated encryption. | Reuse the loader and primitive, but add a voter-attribute codec rather than storing one Hasura `secret` row per voter. |
| Hasura secret precedent | `packages/windmill/src/postgres/secret.rs` and `hasura/metadata/databases/backend-db/tables/sequent_backend_secret.yaml` keep encrypted bytes out of Hasura permissions. | Keep the same server-only boundary: the browser and Hasura must never receive stored ciphertext. |
| User-profile metadata | `packages/sequent-core/src/types/keycloak.rs` already carries Keycloak annotations in `UserProfileAttribute`; `packages/sequent-core/src/services/keycloak/user.rs` reads the profile configuration. | Use a profile annotation as the authoritative secret classification. |
| User read/write API | `packages/harvest/src/routes/users.rs` and `packages/sequent-core/src/services/keycloak/user.rs` currently pass all attributes through ordinary `User` objects. | Partition, protect, and redact at a central boundary; route-specific fixes alone are too easy to bypass. |
| Voter list queries | `packages/windmill/src/services/users.rs` reads, filters, and sorts `user_attribute.value` directly. | Secret names must be rejected from filtering/sorting and omitted before a `User` crosses an external boundary. |
| Admin Portal | `packages/admin-portal/src/resources/User/EditUserForm.tsx` renders fields from profile metadata; `ListUsers.tsx` creates columns and filters from the same metadata. | Render a dedicated secret control and remove secret attributes from generic list columns/filters. |
| Communication variables | `packages/windmill/src/tasks/send_template.rs::get_variables` currently exposes every user attribute under both `user.<name>` and `user.attributes.<name>`. | Replace direct exposure with a common voter-variable resolver that merges only authorized, declared decrypted values. |
| Per-voter reports | `packages/windmill/src/services/reports/template_renderer.rs` builds Handlebars maps; voter-specific implementations include `voter_information_letter.rs`, `manual_verification.rs`, and `ballot_receipt.rs`. | Add the same `user` context to eligible per-voter renderers without changing their existing variables. Never add it to aggregate reports. |
| Bulk import/export | `packages/windmill/src/services/import/import_users.rs` writes attributes directly to Keycloak tables; `packages/windmill/src/services/export/export_users.rs` exports all configured profile attributes. | Import must encrypt before `COPY`/insert. Normal export must omit secret columns rather than exporting ciphertext. |
| Other writers | Application acceptance, Datafix, reconciliation, cast-vote, voter-letter, and other tasks call `edit_user` or write `user_attribute` directly. | Inventory every writer and force it through the protection boundary or prove that it cannot accept a secret name. |
| Runtime access | `beyond/k8s/apps-config/harvest/env.yaml` already selects `AwsSecretManager`, while Harvest and Windmill use different service-account roles. | Grant Harvest read access to only the environment's `master_secret`; keep Keycloak itself and the Admin Portal without access. |

### 2. Configuration and eligibility

Use this Keycloak User Profile annotation:

```json
{
  "name": "customerReference",
  "displayName": "Customer reference",
  "annotations": {
    "sequent.secret": "true"
  },
  "permissions": {
    "view": ["admin"],
    "edit": ["admin"]
  },
  "multivalued": false
}
```

Add helpers in `sequent-core` that parse both JSON boolean `true` and string `"true"`, matching the existing annotation conventions in `UserService.ts`. They should return a `SecretAttributePolicy` keyed by canonical attribute name.

Validate the configuration before accepting writes or running a migration:

- Secret classification is supported only in an election-event realm, not a tenant/admin realm, in v1.
- The attribute must be a custom `user_attribute`, not `username`, `email`, `firstName`, or `lastName`.
- Maintain a denylist of Step operational attributes: tenant/area identifiers, phone routing, authorized elections, vote weight, voted channel, disable reason, permission labels, OTP/enrollment fields, and any attribute referenced by reconciliation or login configuration.
- The attribute must be admin-only in the Keycloak profile and must not be `hidden` from the Admin Portal.
- It must not be used by a Keycloak authenticator, registration/update action, identity-provider linker, protocol/token mapper, uniqueness/search rule, or prefill/update flow. Add a realm configuration check for known Sequent extension settings such as `search-attributes`, `unique-attributes`, `update-attributes`, and `unset-attributes`.
- Initially disallow value-dependent Keycloak validators and Keycloak-required semantics on secret attributes. Keycloak validates the stored ciphertext, not the submitted plaintext. If required/plaintext validation is needed later, implement it in Step with separate secret-specific metadata.
- Reject attempts to remove the annotation or rename the attribute while encrypted values exist. These operations require an explicit migration.

Expose the classification to the Admin Portal through the already-returned `annotations`; do not add a second configuration database.

### 3. Encryption envelope and service boundary

Create a focused module such as `packages/windmill/src/services/voter_secret_attributes.rs`. Harvest already depends on the Windmill service crate, so both synchronous Admin Portal actions and workers can use the same implementation.

The codec should:

1. Load the existing `master_secret` with `vault::get_master_secret()`.
2. Derive a domain-separated 256-bit key with HKDF-SHA-256. Include a fixed domain (`step/keycloak-voter-attribute/v1`), tenant ID, election-event ID, Keycloak user ID, and attribute name. This still uses the existing master secret while preventing cross-domain use and making a ciphertext copied to another voter, event, or attribute fail authentication.
3. Encrypt each logical attribute value independently with the existing ChaCha20-Poly1305 primitive in `strand::symm`. Random nonces preserve semantic security; do not introduce deterministic encryption for searching.
4. Store a compact string envelope such as `seqenc:v1:<base64url(nonce+ciphertext+tag)>` in `user_attribute.value`.
5. Preserve Keycloak's `Vec<String>` shape for multi-valued fields. Do not bind encryption to a value's array index because Keycloak does not guarantee a stable row order.
6. Recognize only the exact prefix/version. A missing prefix on a configured secret is `legacy_plaintext`, not an implicitly valid clear value. An invalid tag, unknown version, or scope mismatch is an error; never return the stored string as a fallback.

Before fixing the plaintext limit, run a compatibility spike against the deployed Keycloak version and schema:

- Confirm the maximum `user_attribute.value` size through both the Admin REST API and direct import path.
- Confirm how the Admin REST API applies User Profile validation to an envelope during create and update.
- Calculate and enforce a server-side plaintext byte limit after base64/envelope expansion, with the same hint in the Admin Portal and import errors.
- Confirm that an unrelated Keycloak user update preserves encrypted attributes byte-for-byte.

The codec API should use scoped types rather than raw strings where practical (`PlainSecretValue`, `EncryptedAttributeValue`, `SecretAttributeScope`) to prevent accidental double-encryption or rendering of ciphertext.

### 4. Permissions and authorization

Add these permissions consistently to:

- `packages/sequent-core/src/types/permissions.rs`
- `packages/sequent-core/src/wasm/wasm_permissions.rs`
- `packages/admin-portal/src/types/keycloak.ts`
- Admin Portal permission translations
- Hasura action permission metadata
- `beyond/k8s/charts/client-setup/templates/admin-tenant-config.yaml`
- Janitor/default Keycloak realm templates and the intended composite admin roles

| Operation | Required permissions |
|---|---|
| See whether a value is set | `voter-read` only; return presence/count metadata, never a value. |
| Reveal one value | `voter-read` **and** `voter-secret-attribute-read`. |
| Create/replace/clear | Normal create/write permission **and** `voter-secret-attribute-write`. |
| Import a file containing a secret column | `voter-import`/`voter-create` **and** `voter-secret-attribute-write`. |
| Render/preview/send an output declaring secrets | Existing output permission **and** `voter-secret-attribute-read`. |
| Normal list/export/send without declared secrets | No new permission. |
| Migrate existing values | A service-only operation or a dedicated break-glass permission; not a normal Admin Portal action. |

Do not rely only on Hasura's active role. Harvest's `authorize` already verifies that every required permission is present in `allowed_roles`, so enforce the conjunction in Harvest before enqueueing work.

### 5. API and data contracts

#### 5.1 External user responses

Extend the user action output with presence-only metadata, for example:

```graphql
type VoterSecretAttributeState {
  name: String!
  is_set: Boolean!
  value_count: Int!
}

type KeycloakUser {
  # existing fields
  attributes: jsonb
  secret_attributes: [VoterSecretAttributeState!]!
}
```

For every external user response:

- Remove configured secret names from `attributes` entirely.
- Populate `secret_attributes` from ciphertext presence only.
- Never use a ciphertext or a reusable redacted placeholder as an attribute value. A placeholder is easy for a read/modify/write client to encrypt as if it were a new secret.
- Apply sanitization to `get_users`, `get_user`, create/edit responses, task results, application responses, and errors—not only the Admin Portal list.

Add one dedicated synchronous action such as `get_voter_secret_attribute(tenant_id, election_event_id, user_id, name)`. It validates realm scope, verifies that the name is currently configured as secret, checks both read permissions, decrypts only that name, emits a value-free audit event, and returns the values. Configure the client query as `no-cache`; clear component state on blur, close, navigation, permission change, and failed refresh.

#### 5.2 Write semantics

Do not mix clear secret values into the generic `attributes` object. Extend create/edit inputs with an explicit operation map:

```json
{
  "secret_attributes": {
    "customerReference": {"operation": "set", "values": ["ABC-123"]},
    "oldReference": {"operation": "clear"}
  }
}
```

Semantics are intentionally three-state:

- missing name: preserve the current encrypted value;
- `set`: validate plaintext, encrypt, and replace all stored values;
- `clear`: remove all rows for that name.

Reject a configured secret name if a caller puts it in ordinary `attributes`. Reject a non-secret name if a caller puts it in `secret_attributes`. This makes accidental plaintext writes visible instead of silently accepting them.

### 6. Write paths

Implement a single partition/protect function and use it everywhere a voter is created or changed.

#### Admin create

1. Load and validate the event realm's secret-attribute policy.
2. Partition ordinary and secret inputs and authorize secret writes only when present.
3. Create the user disabled with ordinary attributes, obtain the final Keycloak user ID, encrypt the secret values with that ID in scope, write them, then apply the requested enabled state.
4. If the secret write fails, compensate by deleting the newly created disabled user and return a value-free error. The compatibility spike may replace this with one atomic/supplied-ID operation if the deployed Keycloak API supports it safely.

#### Admin edit

1. Fetch the existing stored attributes internally.
2. Preserve untouched ciphertext; encrypt only explicit `set` operations; remove explicit `clear` operations.
3. For Datafix election events, perform encryption in Harvest **before** placing the edit on Celery so RabbitMQ and task payloads never carry plaintext.
4. Remove or redact current debug logging in `KeycloakAdminClient::create_user`, `edit_user_with_credentials`, application acceptance, template rendering, and any task that prints full users/attributes.

#### Bulk import

Refactor `import_users.rs` so it loads the profile policy once and classifies CSV headers before processing rows:

- Pass a `may_write_secret_attributes` authorization decision from Harvest into the task; workers must not infer the initiating user's permission.
- Generate/know each Keycloak user ID before protecting its secret cells, then encrypt in Rust before the temporary table/COPY stage.
- Keep secret values out of generated SQL, tracing return values, task logs, row errors, and metrics labels.
- Validate every row before committing the Keycloak transaction; report only row number and attribute name on failure.
- Treat the uploaded source CSV as sensitive because it contains plaintext. Keep it private, define a short retention/deletion policy after import, and document that backups/object versions can otherwise retain the plaintext.

#### Other writers

Audit and update at least these current paths:

- `packages/windmill/src/services/application.rs`
- `packages/windmill/src/services/external/api_datafix.rs`
- `packages/windmill/src/services/external/reconciliation/bulk_create.rs`
- `packages/windmill/src/services/external/reconciliation/apply.rs`
- `packages/windmill/src/tasks/edit_user.rs`
- any cast-vote, enrollment, voter-letter, or Keycloak extension path that updates attributes

Operational writers should normally be unable to address secret names. If a business flow genuinely needs to set one, give it an explicit trusted service capability and route it through the codec. Add a repository test/lint that flags new direct `user_attribute` writes and direct generic Keycloak user writes for review.

### 7. Read, list, filter, and export paths

Add a central `sanitize_user_for_external_response` function. It receives the profile policy and produces a public user plus presence metadata. Use it immediately before serialization rather than expecting every caller to remember individual names.

In `packages/windmill/src/services/users.rs`:

- Reject secret attribute names in dynamic filters and sorts at the backend even if a client manually crafts the request.
- Keep ciphertext available only to internal code that explicitly asks for stored attributes.
- Consider separate `StoredUser` and `PublicUser` types; this is safer than letting the same `User` type mean ciphertext, plaintext, and redacted data in different callers.

In normal CSV exports:

- Filter secret attributes out of both headers and records.
- Do not emit blank secret columns, presence flags, redaction strings, or ciphertext.
- Keep decrypted export behind a separate explicit option requiring both the ordinary export/read permission and `voter-secret-attribute-read`.
- Mark the generated document as containing voter secrets and re-check secret-read permission when issuing its download URL; `voter-export` alone is never sufficient.

Reconciliation and aggregate queries should continue to operate only on non-secret operational attributes. The configuration validator prevents an operator from encrypting one of their dependencies.

### 8. Admin Portal

In `EditUserForm.tsx` and its helper components:

- Detect `sequent.secret` from the profile annotations and render a dedicated `SecretVoterAttributeInput`, not the generic text/date/select control.
- Keep presence state, revealed values, and pending write operations outside the ordinary `IUser.attributes` object and outside review/debug serialization.
- With no secret permission, omit the field or show only non-interactive “value set” metadata according to product preference.
- With write but not read permission, allow **Replace** and **Clear** without reveal.
- With read permission, provide an explicit **Reveal** action, an obvious sensitive-data state, and **Hide**. Do not reveal all fields at once or automatically reveal on form load.
- Review screens must say “Secret value replaced/cleared” and never echo old or new values.
- A save that changes only ordinary fields must omit all secret operations and therefore preserve stored ciphertext.

In `ListUsers.tsx`:

- Remove secret attributes from generated columns, filters, sorting, bulk actions, and copied/exported data.
- Do not fetch revealed values for a list. If presence is displayed, use only `secret_attributes.is_set`.

Add localized labels, permission descriptions, accessible reveal/hide controls, and tests ensuring secrets do not remain in Apollo cache or component state after closing the editor.

### 9. Voter-level report and communication variables

Create a common `VoterTemplateVariableResolver` used by `send_template.rs` and by the report `TemplateRenderer` pipeline.

Each template/report configuration must carry an explicit allowlist, for example:

```json
{
  "secret_attribute_names": ["customerReference"]
}
```

The allowlist is required even when the operator has read permission. It gives the worker an auditable declaration, avoids decrypting every secret for every voter, and prevents dynamic Handlebars `lookup` expressions from bypassing static template inspection.

At enqueue/save time:

- Verify that every declared name is currently a configured secret attribute.
- Require `voter-secret-attribute-read` in addition to the existing send/generate/preview permission when the list is non-empty.
- Persist/pass names and the authorization decision, never plaintext, through scheduled-event and Celery payloads.
- A template editor can show eligible variables with a lock icon and update the declaration when one is inserted. Manual template edits must be validated on save.

At execution time, for each voter:

1. Load the current profile policy and revalidate the declaration; fail closed if it changed.
2. Fetch stored values for only the declared names.
3. Decrypt with the exact tenant/event/user/name scope.
4. Merge values into the existing structure:
   - first value at `user.<attribute>`;
   - full array at `user.attributes.<attribute>`.
5. Render and immediately drop the clear-value context.

An unset declared value behaves like an unset ordinary attribute. A corrupt or legacy plaintext value is an execution error, not a missing variable and not printable fallback text.

For the report framework, add the common `user` object after typed `UserData` is converted to a Handlebars map when `get_voter_id()` is present and the report declares secret names. This enables voter information letters and other true per-voter reports without adding one Rust field per custom attribute. Do not inject voter data into participation, activity, tally, results, or other aggregate/system reports.

Remove the current debug logging of complete `user_data_map` and rendered report HTML in `template_renderer.rs`. Keep the existing `send_template` rule that electoral logs contain delivery metadata but not rendered bodies. Mark generated documents containing secret variables as sensitive/private and apply the existing document/report encryption policy where configured.

### 10. Migration and lifecycle

Add an idempotent migration/maintenance command with dry-run support and explicit tenant, election event, and attribute scopes. It should report counts only:

- missing values;
- already-encrypted v1 values;
- legacy plaintext values to encrypt;
- invalid/unknown envelopes;
- successfully migrated and failed rows.

Safe rollout for an existing plaintext attribute:

1. Deploy code, permissions, redaction, and codec support with no secret annotations active.
2. Grant only the intended roles and verify Harvest/Windmill master-secret access.
3. Mark the attribute secret. From this point normal reads redact it and new writes encrypt it; reveal/report use fails for legacy rows.
4. Run the scoped migration. It is the only code path allowed to interpret an unprefixed value as legacy plaintext.
5. Verify that zero legacy values remain, sample authorized reveal/report behavior, then enable templates that declare the field.

Provide inverse re-encryption tooling before supporting removal/rename. Do not let an operator simply remove the annotation, which would expose envelopes through normal APIs and break consumers.

The current master-secret system has no independent key-ring/rotation mechanism. Record that operational constraint explicitly: rotating `master_secret` already requires coordinated re-encryption of Hasura secrets and will now also require re-encrypting voter attributes. The envelope version makes a future key-ring migration possible, but silent rotation is out of scope for v1.

### 11. Deployment, security, and observability

- In Beyond/IAM, grant the Harvest service-account role `GetSecretValue` for exactly `<AWS_SM_KEY_PREFIX>master_secret`. Windmill roles already use the vault; verify every worker queue that can render voter-level output. Do not grant the Keycloak pod, Hasura, Admin Portal, or browser access.
- Make Harvest and relevant Windmill readiness checks fail when an event has active secret attributes but the master secret cannot be loaded. Do not auto-create a new production master key from a workload that merely lost read access.
- Add metrics for encrypt/decrypt operations, failures by reason/version, legacy-value counts, and authorized reveal/report-use counts. Labels may include environment and attribute name only if policy permits; never include voter identifiers or values.
- Add structured, value-free audit records for reveal, set, clear, import, migration, and report use. For bulk sends/imports, audit the job, actor, declared field names, and counts rather than one sensitive event per value.
- Review all `#[instrument]`, `Debug`, `info!`, task-error, and provider-error paths. Use `skip_all` or redacted custom `Debug` implementations on any type that can hold clear values.
- Ensure application traces, Sentry/error reporting, RabbitMQ payloads, task execution logs, immutable electoral logs, and test snapshots never receive plaintext.

### 12. Test plan

#### Unit tests

- Envelope round trip, random nonce/non-determinism, multi-value handling, empty/Unicode values, and size limit.
- Authentication failure for modified bytes, wrong tenant/event/user/attribute, unknown version, and unprefixed values.
- Annotation parsing and every configuration eligibility rule.
- Partitioning and three-state preserve/set/clear behavior.
- Public-user sanitization never emits plaintext or ciphertext.
- Template resolver exposes only declared names in both supported variable shapes and preserves collision rules for canonical user fields.

#### Authorization tests

- Matrix for ordinary read/write, secret read only, secret write only, both, super-admin composites, and missing base permission.
- Crafted GraphQL/HTTP requests cannot put a secret in ordinary attributes, filter/sort by it, reveal another realm's user, or add an undeclared report variable.
- Scheduled/Celery execution cannot gain access merely because the worker service account can decrypt.

#### Integration tests

- Admin create/edit/reveal/hide/preserve/replace/clear against a real Keycloak realm.
- Unrelated voter edits preserve ciphertext.
- Bulk import stores envelopes and normal export omits the columns.
- Datafix/reconciliation paths either protect or explicitly reject secret names.
- Email, SMS, voter information letter, and one additional per-voter renderer receive declared clear values; aggregate reports do not.
- Corrupt and legacy rows fail closed without values in task or application logs.
- Migration dry run, first run, retry/idempotency, and post-migration verification.

#### Security checks

- Search Keycloak DB, Hasura DB, S3 outputs, RabbitMQ, logs, traces, browser network/cache, generated reports, and exported CSVs for a canary plaintext value.
- Confirm ciphertext size/validation behavior on the exact production Keycloak version.
- Confirm IAM least privilege and behavior when Secrets Manager is unavailable.

### 13. Delivery order

| Phase | Deliverable |
|---|---|
| 0 | Keycloak storage/API compatibility spike; writer inventory; threat-model review; final envelope and size limit. |
| 1 | Shared policy/codec, scoped types, sanitization, permissions, IAM, audit/metrics, and unit tests. No attributes enabled yet. |
| 2 | Harvest create/edit/reveal APIs and Admin Portal masked/reveal/replace/clear UX. |
| 3 | Bulk import protection, normal export omission, and all non-UI writer protections/rejections. |
| 4 | Common voter template resolver, explicit declaration schema, email/SMS integration, then per-voter report integration. |
| 5 | Migration command, runbook, end-to-end canary, staged rollout to one non-production event, then production enablement. |

The feature is complete only when every direct Keycloak attribute writer and every external user serializer is covered, not merely when the Admin Portal and `send_template` happy paths work.

## Part 3 — Product ticket

### Title

Protect confidential voter fields while allowing their use in voter communications and reports

### Summary

Allow selected custom voter fields to be classified as confidential. Their values must be securely stored, hidden from normal voter views and exports, and available only to specifically authorized users and voter-level communications or reports.

### Problem

Some elections need to keep additional information for each voter, such as a customer reference, membership number, personal identifier, or other private value. Today, custom voter fields are treated like ordinary voter data. This makes it difficult to store information that should not be visible to every administrator who can access the voter list.

At the same time, these values may be needed in personalized emails, SMS messages, letters, or other outputs sent to an individual voter. Election teams need to use them without making them generally visible across the product.

### Goal

Election administrators can designate eligible custom voter fields as confidential. The product protects those values by default while allowing controlled viewing, editing, importing, and use in approved voter-level outputs.

### User stories

- As an election configurator, I want to mark a custom voter field as confidential so that its value is protected throughout the product.
- As an authorized administrator, I want to see whether a confidential value exists without exposing it unnecessarily.
- As an authorized administrator, I want to reveal a confidential value when I have a legitimate operational need.
- As an authorized administrator, I want to add, replace, or clear a confidential value without revealing its previous value.
- As an election operator, I want approved voter communications and reports to use confidential fields as personalized variables.
- As a security or audit user, I want sensitive actions to be traceable without the confidential values appearing in audit records.

### Scope

#### Included

- Classify eligible custom voter fields as confidential at election-event level.
- Store confidential values securely.
- Show whether each confidential value is set without showing the value itself.
- Provide separate permissions for viewing and managing confidential voter fields.
- Allow authorized users to reveal, replace, clear, and import confidential values.
- Allow approved email, SMS, letter, and other voter-level templates to use declared confidential fields.
- Hide confidential fields from ordinary voter lists, searches, filters, sorting, bulk actions, and standard voter exports.
- Allow an explicit, warned decrypted export only for users with confidential-field viewing permission, and preserve that permission on document download.
- Record value-free audit information when a confidential value is revealed, changed, cleared, imported, or used in an output.
- Preserve all existing behavior for elections that do not configure confidential fields.

#### Not included

- Making standard voter identity or operational fields confidential, including name, username, email address, mobile number, voting area, voting status, or fields used to identify/authenticate a voter.
- Using confidential fields to search for, sort, filter, authenticate, match, or deduplicate voters.
- Allowing voters to view or edit these fields themselves.
- Including confidential values in standard voter exports without selecting the restricted decrypted-export option.
- Allowing confidential values in aggregate, election-wide, tally, results, or activity reports.
- Changing the appearance or behavior of existing non-confidential voter fields.

### Permissions

Introduce two independently assignable capabilities:

| Capability | Product behavior |
|---|---|
| View confidential voter fields | The user may explicitly reveal a confidential value and run an approved voter-level output that uses it. |
| Manage confidential voter fields | The user may add, replace, clear, or import confidential values without automatically receiving permission to reveal them. |

These capabilities supplement the user's existing voter and output permissions; they do not replace them.

### Expected user experience

#### Voter list

- Confidential fields are not available as columns, filters, sorting options, or searchable fields.
- Confidential values never appear in list results or bulk actions.
- If presence is useful, the product may show only a neutral status such as **Value set**.

#### Create or edit voter

- A confidential field is clearly identified as protected.
- The existing value is masked and is not loaded automatically.
- Users with management permission can choose **Set**, **Replace**, or **Clear**.
- Leaving the field untouched preserves its current value.
- A user can replace or clear a value without first revealing it.
- Confirmation/review screens say that a protected value was added, replaced, or cleared but never display the old or new value.

#### Reveal

- Revealing is an explicit action, not the default state of the form.
- Only users with viewing permission see the **Reveal** action.
- A revealed value can be hidden again and is removed from the screen when the user closes or leaves the voter record.
- The reveal action is audited without recording the value.

#### Imports

- Authorized imports may contain confidential fields in the same way they contain other custom voter fields.
- Users without management permission receive a clear error if an import includes a confidential field.
- Errors identify the affected row and field but never repeat its value.

#### Communications and voter-level reports

- Template authors can see which available variables are confidential.
- A template must explicitly declare which confidential fields it uses.
- Sending, generating, scheduling, or previewing an output that uses confidential fields requires viewing permission.
- Only the declared confidential fields are made available to that output.
- An output that does not declare confidential fields behaves exactly as it does today.
- Missing confidential values behave like other missing optional voter data; an unreadable or invalid protected value stops the operation safely.

### Acceptance criteria

1. **Configure a confidential field**
   - Given an eligible custom voter field,
   - when it is classified as confidential,
   - then newly entered values are securely stored and are not visible through ordinary voter access.

2. **Default protection**
   - Given a voter with a confidential value,
   - when a user opens the voter list or voter record with ordinary voter permissions,
   - then the value is not displayed, searchable, filterable, sortable, copyable, or exportable.

3. **Presence without disclosure**
   - Given a voter with a confidential value,
   - when an administrator opens the voter record,
   - then the product can indicate that the value is set without revealing it.

4. **Authorized reveal**
   - Given a user with both ordinary voter access and permission to view confidential fields,
   - when the user explicitly selects **Reveal**,
   - then the value is shown only in that voter record and the action is audited without the value.

5. **Unauthorized reveal**
   - Given a user without permission to view confidential fields,
   - when the user accesses a voter record or attempts a direct reveal,
   - then the value is not returned.

6. **Manage without reveal**
   - Given a user with permission to manage confidential fields but not view them,
   - when the user replaces or clears a value,
   - then the change succeeds without showing the previous value.

7. **Preserve untouched values**
   - Given an existing confidential value,
   - when an authorized user changes another part of the voter record and leaves the confidential field untouched,
   - then the confidential value is preserved.

8. **Authorized import**
   - Given an import containing a configured confidential field and an authorized importer,
   - when the import completes,
   - then the values are protected and do not appear in task logs or standard exports.

9. **Voter-level output**
   - Given an approved voter-level template that declares a confidential field and an authorized operator,
   - when the output is generated or sent,
   - then the correct voter value is available to the template and no undeclared confidential field is available.

10. **Unauthorized output**
    - Given a template that uses a confidential field and an operator without viewing permission,
    - when the operator attempts to preview, generate, schedule, or send it,
    - then the operation is refused without exposing the value.

11. **Standard export**
    - Given voters with confidential values,
    - when a standard voter export is generated,
    - then confidential columns and values are absent from the export.

12. **Backward compatibility**
    - Given an election with no confidential fields configured,
    - when administrators manage voters or generate outputs,
    - then the behavior and appearance remain unchanged.

### Product safeguards

- Confidential values must never appear in application logs, task logs, error messages, audit contents, or standard exports.
- Viewing and management permissions must be assignable separately.
- Confidential values must be exposed only for the individual voter and purpose currently being handled.
- Invalid or unavailable protected data must fail safely rather than being displayed in its stored form.
- Existing values must be protected before the field is used in production communications or reports.

### Definition of done

- Product, security, and engineering agree on which field types are eligible.
- Permission names and default role assignments are approved.
- Admin Portal behavior is reviewed for users with no confidential permission, view only, manage only, and both permissions.
- At least email, SMS, and one voter-level document/report support declared confidential variables.
- Standard voter lists and exports are verified not to disclose confidential values.
- Audit behavior and operational rollout guidance are documented.
- The feature is validated in a non-production election before being enabled for production data.
