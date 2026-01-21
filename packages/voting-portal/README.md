<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# voting-portal

Voter-facing interface for the Sequent Voting Platform. Provides a secure,
accessible, and user-friendly voting experience with end-to-end verifiability.

## Features

- Secure ballot casting with cryptographic guarantees
- Multi-language support
- Accessibility compliance (WCAG 2.1)
- Ballot verification and audit capabilities
- Responsive design for desktop and mobile devices

## Development

From the monorepo root in a Dev Container:

```bash
cd packages/
yarn && yarn build:ui-core && yarn build:ui-essentials  # first time only
yarn start:voting-portal
```

The voting portal will be available at http://127.0.0.1:3000/

## Architecture

Built with React and TypeScript, integrating with:
- **Hasura GraphQL API** for data operations
- **Keycloak** for voter authentication
- **ui-essentials** for shared UI components
- **sequent-core** (compiled to WASM) for client-side ballot encryption and verification

## Documentation

For detailed documentation, see:
- [Voting Portal Guide](https://docs.sequentech.io/docusaurus/main/docs/voting_portal/voting_portal)
- [Developer Documentation](https://docs.sequentech.io/docusaurus/main/docs/developers/Voting-Portal/developers_voting-portal)
