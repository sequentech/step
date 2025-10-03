---
id: release-8.7.6
title: Release Notes v8.7.6
---
<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## 🐞 Keycloak voter logs are not recorded

Voter logs related to Keycloak (login, login error, code to token) were being 
published to the wrong rabbitmq queue. This has been fixed and now they are 
published to the queue for the respective environment.

- Issue: [#7750](https://github.com/sequentech/meta/issues/7750)
- Main commit: `fb8c8dda2`
- Cherry-picks:
  - release/8.7: `51f7743b9`
  - release/9.0: `5a373cecf`
  - release/9.1: `77f58f56f`

## 🐞 Voters can't login to election events in new tenants

For security, secrets/certificates are generated randomly when creating a new
election event/tenant. However the secret for the service account of the tenant
should be set by the system as it is used internally. This is now set by
environment variables  `KEYCLOAK_CLIENT_ID` and `KEYCLOAK_CLIENT_SECRET`.

- Issue: [#7740](https://github.com/sequentech/meta/issues/7740)
- Main commit: `d3bfd52c3`
- Cherry-picks:
  - release/8.7: `ce3757166`
  - release/9.0: `939e5c0a3`
  - release/9.1: `434681064`
