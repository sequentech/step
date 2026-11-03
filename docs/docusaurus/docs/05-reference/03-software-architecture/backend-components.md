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

### wbraid
- **Role**: Native cryptographic implementation, DKG, mixnet, proofs and signatures.
- **Path**: `step/packages/wbraid`
- **Integration**: `voting-crypto`, `protocol-board` and the native trustee runner.
- **Storage**: `b4v6` for protocol messages; independent immudb storage for application and database audit logs.

---
