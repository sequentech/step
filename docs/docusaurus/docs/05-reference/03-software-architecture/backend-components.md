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
envelope and does not have access to the plaintext or the master secret.

The encryption key is derived from the existing Step `master_secret` and the tenant, election
event, voter ID, and attribute name. Consequently, moving an envelope to another voter, event, or
attribute does not produce a valid value. Envelopes are randomized, so secret fields cannot be
searched, sorted, filtered, or compared by their encrypted representation.

Harvest enforces the synchronous create, edit, reveal, and permission boundaries. Windmill uses the
same codec for voter imports, explicitly decrypted exports, communications, and per-voter reports.
Ordinary user responses and exports remove secret fields rather than returning ciphertext.

Only Harvest and the Windmill workers that perform these operations should be able to read the
configured `master_secret`. Keycloak, Hasura, the Admin Portal, and browser clients must not receive
that credential. Losing or replacing the master secret makes existing voter envelopes unreadable;
master-secret rotation therefore requires a coordinated re-encryption migration and must not be
performed as an isolated secret replacement.
