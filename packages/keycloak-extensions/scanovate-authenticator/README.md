<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Scanovate Authenticator

Keycloak authenticator (`scanovate-authenticator`) that verifies the voter's
identity with Scanovate B-Trust during enrollment, following the B-Trust v3.8.2
Identity Verification Tech Specs.

See the [Scanovate Identity Verification guide](../../../docs/docusaurus/docs/integrations/scanovate_identity_verification_guide.md)
for the configuration reference and how to test the integration.

## Tests

```bash
cd packages/keycloak-extensions
mvn -B -pl scanovate-authenticator -am verify
```

The e2e mock server (`packages/e2e/src/mock_server`) implements the B-Trust
endpoints used by the authenticator, so the whole flow can be exercised
locally without B-Trust credentials.
