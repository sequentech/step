---
id: cli_tutorials_getting_started
title: Getting Started with Step CLI
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


This is a command-line tool for automating and streamlining operations within the Sequent Voting Platform ecosystem.

### Features

- Election event and contest creation
- Voter management and import
- Key ceremony and tally management
- Load testing and data generation utilities
- Template rendering for email notifications
- ImmuDB bulletin board export

### Installation

Build from source:

```bash
cd packages/step-cli
cargo build --release
```

#### Get help:

```bash
cargo run step --help
cargo run step <command> --help
```

### Configuration

Configure CLI with your environment credentials:

```bash
cargo run step config \
  --tenant-id <TENANT_ID> \
  --endpoint-url <ENDPOINT_URL> \
  --keycloak-url <KEYCLOAK_URL> \
  --keycloak-user <KEYCLOAK_USER> \
  --keycloak-password <KEYCLOAK_PASSWORD> \
  --keycloak-client-id <KEYCLOAK_CLIENT_ID> \
  --keycloak-client-secret <KEYCLOAK_CLIENT_SECRET>
```
Parameters:
- endpoint-url: use `HASURA_ENDPOINT` .env variable
- keycloak-url: use `KEYCLOAK_URL` .env variable
- keycloak-client-id: use `api-key-client` 
- keycloak-client-secret:  use the matching serect for `api-key-client` (found in the Keycloak console)