---
id: backend-components
title: Backend Components
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Backend Services

### Harvest
- **Role**: Custom business logic that complements Hasura.
- **Responsibilities**:
  - Imports/Exports
  - Generating publications
  - Permissions and validation
  - Authorized secret voter-field create, edit, and reveal operations
  - Keys and Tally ceremonies
  - Election status monitoring
  - Third-party integrations
- **Path**: `step/packages/harvest`
- **Technologies**: Rust, Rocket, Hasura (GraphQL Actions)

---

### Windmill
- **Role**: Consumer service that handles background business logic and scheduled tasks.
- **Responsibilities**:
  - Ballots, Ceremonies, Tally Sheets
  - Audit Logs, Key Handling
  - User Management, Statistics
- **Subservices**:
  - `services/ballot_styles`: Ballot display
  - `services/ceremonies`: Key and tally ceremonies
  - `services/consolidation`: Audit logs, result packaging
  - `services/tally_sheets`: Manual tally support
  - `services/vault`: Vault secret management
  - `services/voter_secret_attributes`: Scoped encryption and decryption of secret voter fields
- **Path**: `step/packages/windmill`
- **Technologies**: Rust, Hasura, GraphQL, Postgres, AWS Secret Manager, HashiCorp Vault

---

### Beat
- **Role**: Publishes scheduled events and review board tasks to the queue.
- **Path**: `step/packages/beat`
- **Technologies**: Rust

---

### Velvet
- **Role**: Finalizes elections by decoding ballots, tallying, generating reports, and marking winners.
- **Path**: `step/packages/velvet`
- **Technologies**: Rust

---

### Braid
- **Role**: Implements a verifiable mixnet for anonymizing votes.
- **Technologies**: Rust
- **Path**: `step/packages/braid`

---

### Strand
- **Role**: Core cryptographic library used by Braid.
- **Key Components**:
  - ElGamal & Distributed Key Generation
  - Shuffling (Mixnet)
  - ZKPs (Schnorr, Chaum-Pedersen)
  - Signature & Symmetric encryption
- **Technologies**: Rust
- **Path**: `step/packages/strand`

## Secret Voter Attribute Boundary

An election-event Keycloak User Profile attribute annotated with `sequent.secret=true` is encrypted
before Step writes it to Keycloak. Keycloak stores a versioned `seqenc:v1:` authenticated-encryption
envelope. By default Keycloak has neither the master key nor plaintext access. The explicit
encrypted-attribute login option described below grants the authentication extension the key so
it can decrypt the configured credential server-side; stored attributes remain encrypted.

The encryption key is derived from the existing Step `master_secret` and the tenant, election
event, voter ID, and attribute name. Consequently, moving an envelope to another voter, event, or
attribute does not produce a valid value. Envelopes are randomized, so secret fields cannot be
searched, sorted, filtered, or compared by their encrypted representation.

Harvest enforces the synchronous create, edit, reveal, and permission boundaries. Windmill uses the
same codec for voter imports, authorized opt-in exports, communications, and per-voter reports.
Ordinary user responses and default voter exports remove secret fields rather than returning ciphertext.

Creating a voter with secrets uses two audited writes: Keycloak first validates a disabled voter
with ciphertext under a provisional scope, then Harvest re-encrypts the values against the ID
returned in Keycloak's `Location` header and restores the requested enabled state. If finalization
fails, the voter stays disabled. Required fields are validated on both writes; blank input stays missing.

Every reveal, set, clear, import, decrypted export, and use of a secret attribute in a
communication or per-voter report is recorded in the election event's electoral log as an
admin-signed `VOTER_SECRET_ATTRIBUTE` entry. The entry names the administrator, the voter where
one is involved, the attribute names and, for generated reports, the document id. It never contains a value.
The entry is written before the action takes effect, so an action that cannot be audited does not
happen.

### Voter PIN Link

The [one-time PIN link](../../02-election_managers/01-tutorials/102-admin_portal_tutorials_voter-pin-link.md)
is the one place outside encrypted login where a Keycloak extension decrypts a stored secret. A public
realm resource, off by default and turned on per realm, decrypts the PIN attribute server-side for
the single request that spends the link, and wipes it after writing the response. It needs the same
`MASTER_SECRET`. The envelope remains bound to its voter, event and attribute, so a moved envelope
refuses instead of decrypting.

The link's token is derived, not stored. Windmill and Harvest derive it from the PIN with HMAC-SHA-256
under an HKDF key from the master secret, scoped to tenant, event and voter, and store only its plain
SHA-256 in a generated attribute, because envelopes cannot be searched. The token is random enough
that a plain hash is safe for it, which would not be true of the PIN.

These generated attributes are written by direct SQL and are not declared in the User Profile, so the
account console, tokens and administrative `PUT`s neither see nor change them. Harvest and the voter
import refuse them as input, and templates, reports and default exports strip them.

Mail recipients get the token through `pin_link_token` and `pin_link_url`, which go through
`secret_attribute_names` and therefore keep the `voter-secret-attribute-read` permission and the
communication entry in the electoral log of any other secret attribute.

Each reveal writes a `VOTER_SECRET_ATTRIBUTE: PIN_LINK_REVEAL` entry with no value. Unlike the entries
above it names no administrator, only the voter, because a voter triggers it. The message broker confirms
it inside the database transaction that deletes the hash: if the broker does not confirm, the
transaction rolls back and the link stays alive. If the broker confirms and the commit then fails, the
log shows a reveal the voter did not see, and never the opposite.

Secret-bearing reports use private storage and require secret-read permission at download, including
password retrieval. Ordinary document writers cannot update or delete their access metadata.
Delivery audit entries do not contain rendered subjects or bodies. Explicitly selecting
`EMAIL_TRANSPORT_NAME=Console` or `SMS_TRANSPORT_NAME=Console` enables test-only delivery: the full
rendered message, including decrypted secret values, is printed to the worker console instead of
being sent. Use synthetic data and do not enable Console transport in production. Unknown transport
names fail rather than silently falling back to Console. Real delivery transports do not log bodies.

Harvest and the Windmill workers use the configured `master_secret` for these operations. Keycloak
needs the same 32-byte key, encoded as 64 hexadecimal characters in its `MASTER_SECRET` environment
variable, to verify encrypted login attributes. The supplied Compose files pass one shared
`MASTER_SECRET` deployment value to Keycloak, Harvest and Windmill. Keycloak therefore receives
the key whenever it is configured for the stack, but encrypted-attribute login still requires an
explicit `SECRET_ATTRIBUTE` credential policy; ordinary `PASSWORD` verification remains the default.
For external-vault deployments, inject the same existing vault master key into Keycloak.
Never send the master key to Hasura, the Admin Portal, or browser clients.
Losing or replacing the master secret makes existing voter envelopes unreadable;
master-secret rotation therefore requires a coordinated re-encryption migration and must not be
performed as an isolated secret replacement.

Decrypted exports retain a task-bound authorization grant in PostgreSQL. Workers reload the trusted
task row and check document, tenant, event and expiry before decrypting. The lifetime is controlled
by `WINDMILL_SECRET_EXPORT_GRANT_TTL_SECONDS` (default 86400) on Harvest and Windmill.
Explicitly password-encrypted event archives may include classified S3 documents under the same
grant; ordinary archives still exclude them and unknown/uncommitted document objects.

Generated encrypted reports save the exact password used in a document-bound vault entry and
keep its id in access annotations (not the password). Downloads from Reports and Tasks use the
existing password dialog and permission-checked `get_document_password` API. Older encrypted
reports without this entry still show decryption instructions and use their previously supplied
password. Changing a report configuration does not change earlier documents' passwords.
