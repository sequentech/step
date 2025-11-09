<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# admin-portal

Administration interface for the Sequent Voting Platform. Provides election management, voter administration, and system configuration capabilities.

## Features

- Election event and contest management
- Voter registration and area assignment
- Trustee configuration and key ceremony management
- Tally management and results publication
- Real-time monitoring and audit logs

## Development

From the monorepo root in a Dev Container:

```bash
cd packages/
yarn && yarn build:ui-core && yarn build:ui-essentials  # first time only
yarn start:admin-portal
```

The admin portal will be available at http://127.0.0.1:3002/

## Architecture

Built with React and TypeScript, integrating with:
- **Hasura GraphQL API** for data operations
- **Keycloak** for authentication and authorization
- **ui-essentials** for shared UI components
- **sequent-core** (compiled to WASM) for cryptographic operations

## Documentation

For detailed documentation, see:
- [Admin Portal Guide](https://docs.sequentech.io/docusaurus/main/docs/admin_portal/)
- [Developer Documentation](https://docs.sequentech.io/docusaurus/main/docs/developers/Admin-Portal/developers_admin-portal)
