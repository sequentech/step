<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Scanovate Authenticator

Keycloak authenticator (`scanovate-authenticator`) that verifies the voter's
identity during enrollment with the Scanovate services hosted on premise:
Liveness Plus, Face Match and OCR. Nothing is sent to any third party.

See the [Scanovate Identity Verification guide](../../../docs/docusaurus/docs/integrations/scanovate_identity_verification_guide.md)
for the configuration reference, and
[Scanovate On-Premise Services](../../../docs/docusaurus/docs/integrations/scanovate_on_premise_guide.md)
for how to run the services and test the whole flow.

## Tests

```bash
cd packages/keycloak-extensions
mvn -B -pl scanovate-authenticator -am verify
```

`scripts/configure-realm.sh` points the Scanovate steps of a realm to the
services, see the end-to-end test of the on-premise guide.
