<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# step-cli

Command-line tool for automating and streamlining operations within the Sequent Voting Platform ecosystem.

## Features

- Election event and contest creation
- Voter management and import
- Key ceremony and tally management
- Load testing and data generation utilities
- Template rendering for email notifications
- ImmuDB bulletin board export

## Quick Start

### Installation

Build from source:

```bash
cd packages/step-cli
cargo build --release
```

Or use the pre-compiled binary from GitHub releases.

### Configuration

Configure CLI with your environment credentials:

```bash
step config \
  --tenant-id <TENANT_ID> \
  --endpoint-url <ENDPOINT_URL> \
  --keycloak-url <KEYCLOAK_URL> \
  --keycloak-user <KEYCLOAK_USER> \
  --keycloak-password <KEYCLOAK_PASSWORD>
```

For local development:
- Tenant ID: `90505c8a-23a9-4cdf-a26b-4e19f6a097d5`
- Endpoint URL: `http://graphql-engine:8080/v1/graphql`
- Keycloak URL: `http://keycloak:8090`

### Usage

Get help:

```bash
step --help
step <command> --help
```

Common commands:

```bash
# Create an election event
step create-election-event --name "My Election" --description "Description"

# Import election from JSON
step import-election --file-path /path/to/election.json

# Start key ceremony
step start-key-ceremony --election-event-id <EVENT_ID>

# Generate voters
step generate-voters --working-directory ./data --num-users 1000
```

## Documentation

For comprehensive CLI reference and tutorials, see:
- [CLI Documentation](https://docs.sequentech.io/docusaurus/main/docs/developers/CLI/cli_cli)
- [CLI Reference](https://docs.sequentech.io/docusaurus/main/docs/developers/CLI/Reference/cli_reference)
- [Creating an Election Tutorial](https://docs.sequentech.io/docusaurus/main/docs/developers/CLI/Tutorials/Creating-an-Election-with-the-CLI/cli_tutorials_creating-an-election-with-the-cli)
