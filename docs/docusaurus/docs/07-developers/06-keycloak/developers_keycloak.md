---
id: developers_keycloak
title: Developers Keycloak
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->




## Harvest request authentication

Harvest verifies bearer-token signatures, expiry and issuer before trusting
Hasura authorization claims. The issuer realm must match the token's tenant and
the election-event claim when present. Older event templates may omit that
optional claim; their keys still come from the exact issuer realm. Signing keys are fetched from `KEYCLOAK_URL`
over the configured internal connection and cached for five minutes. Requests
fail authentication when a needed signing key cannot be retrieved, including
while thirty-two key downloads are already in progress.

Harvest requires `KEYCLOAK_URL` and trusts it as an issuer base.
`KEYCLOAK_PUBLIC_URL` is optional and needed when tokens use that public URL as
their issuer; `KIOSK_KEYCLOAK_URL` is likewise optional for kiosk issuers. Deployments using additional frontend aliases must supply
their exact HTTP(S) base URLs in the comma-separated `HARVEST_JWT_ISSUER_URLS`
setting, including any `/auth` prefix. These URLs select accepted issuers;
key downloads always use the internal `KEYCLOAK_URL`. The checked-in local and
air-gapped configurations explicitly include their localhost issuer aliases.

Before restarting Harvest with this change, verify that every normal and kiosk
login issuer is configured and that Harvest can reach each realm's internal
`protocol/openid-connect/certs` endpoint. Existing Keycloak RS256 access tokens
and Datafix client-credential tokens use the same verification boundary. No
database or realm migration is required.
