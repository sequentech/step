<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Scanovate Authenticator

Keycloak authenticator (`scanovate-authenticator`) that verifies the voter's
identity during enrollment with the Scanovate services hosted on premise, as
Scanovate's on-premise kit integrates them: the capture page embeds the OCR and
Liveness Plus pages of the services themselves, which check Keycloak's
one-time tokens and post their results to the `identity-verification` realm
resource (`<ocr|liveness>/verify` and `<ocr|liveness>/callback`), server to
server. With the `holding-photo` policy `REQUIRED` (the default), the capture
page then takes a photo of the voter holding their document and uploads it to
`capture/holding` with a one-time token. Keycloak then compares the live face
with the portrait OCR read and with the holding photo using Face Match, and
applies the configured rules to what OCR read. Nothing is sent to any third
party.

See the [Scanovate Identity Verification guide](../../../docs/docusaurus/docs/integrations/scanovate_identity_verification_guide.md)
for the configuration reference, and
[Scanovate On-Premise Services](../../../docs/docusaurus/docs/integrations/scanovate_on_premise_guide.md)
for how to run the services and test the whole flow.

See [Failure diagnostics](../../../docs/docusaurus/docs/07-developers/06-keycloak/scanovate_diagnostics.md)
for the event-log field contract, redactions, support reference and contact-lookup design gap.

## Tests

```bash
cd packages/keycloak-extensions
mvn -B -pl scanovate-authenticator -am verify
```

`scripts/configure-realm.sh` points the Scanovate steps of a realm to the
services, see the end-to-end test of the on-premise guide.
