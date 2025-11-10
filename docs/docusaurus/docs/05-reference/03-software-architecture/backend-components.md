---
id: backend-components
title: Backend Components
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Backend Services

### Harvest
- **Role**: Custom business logic that complements Hasura.
- **Responsibilities**:
  - Imports/Exports
  - Generating publications
  - Permissions and validation
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

