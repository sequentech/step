---
id: cli_tutorials_getting_started
title: Getting Started With the CLI
position: 1
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The CLI (Command-Line Interface) is a tool used to execute and test actions within the Sequent Voting Platform ecosystem.
This guide walks you through building the CLI, configuring it, and authenticating with a valid Keycloak user.

## Step 1: Build the CLI Package
Navigate to the CLI package directory and build it:

```bash
cd packages/step-cli
cargo build --release
```

----

## Step 2: Create Initial Configuration & Authenticate
Create the initial CLI configuration and authenticate using a valid Keycloak admin user for the tenant.

```bash
cli step config --tenant-id 90505c8a-23a9-4cdf-a26b-4e19f6a097d5 \
--endpoint-url http://graphql-engine:8080/v1/graphql \
--keycloak-url http://keycloak:8090 \
--keycloak-user admin \
--keycloak-password admin \
--keycloak-client-id api-key-client \
--keycloak-client-secret 4lzmxNgZHjfzS5BwDVlyrRUDqwvFLUvL
```
### What This Does
* Creates a local CLI configuration file
* Authenticates against Keycloak
* Stores the access credentials for future CLI commands

> ⚠️ Note:
> Most commands require an admin user, but this may vary depending on the specific action being executed.

---
## Troubleshooting Tips
* Ensure the Keycloak realm matches the tenant you are configuring
* Verify the GraphQL endpoint is reachable
* Confirm the client ID and secret are valid
* Check that the user has the required permissions

---
## Next Steps
After configuration, you can start using the CLI to:
* Create election events
* Manage voters
* Execute Keys/Tally ceremony
* Load testing and data generation utilities
* Template rendering for email notifications
* Exporting ImmuDB bulletin board