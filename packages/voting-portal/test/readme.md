<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

## Running voting-portal tests

### Admin portal
- create desired election event, areas and voter
- assign contests to election area 
- configure voter auth credentials and assign area to voter
- send voter notification email and get login url from windmill logs
- initialize and run voting-portal via codespace (```yarn && yarn build:ui-essentials && yarn start:voting-portal```)
- manually log in via login url to initialize voter account by changing default password and verifying email via otp
- use the delivered email verification code; dummy couriers require a test code configured only in an isolated development realm
- configure ./index.ts with loginUrl, voter email and voter updated password

### Keycloak
- disable 2fa for election event via keycloak admin interface

### Run test
- cwd /voting-portal in local workspace(not codespace)
- npx nightwatch path/t0.test.ts

### Login hint browser matrix

Configure each variable with a Voting Portal tenant/event URL. The stock registration
realm must include the `login-hint-registration-prefill` action. The deferred realms
must configure `prefill-parameters-policy` as `IGNORE` and `ACCEPT`, respectively.

Per-attribute behaviour comes from the `loginHintPrefillPolicy` user profile
annotation (`EDITABLE`, `READ_ONLY` or `IGNORE`, defaulting to `EDITABLE`).

- `PREFILL_STOCK_LOGIN_URL`: Voting Portal `/login` URL using the stock username form
- `PREFILL_STOCK_REGISTRATION_URL`: Voting Portal `/enroll` URL using stock registration
- `PREFILL_REDIRECT_REGISTRATION_URL`: Voting Portal `/login` URL whose flow redirects to registration
- `PREFILL_DEFERRED_IGNORE_URL`: Voting Portal `/enroll` URL using deferred registration with `IGNORE`
- `PREFILL_DEFERRED_ACCEPT_URL`: Voting Portal `/enroll` URL using deferred registration with `ACCEPT`

Run `yarn test:login-hints:e2e`. Scenarios without a configured URL are reported
as skipped. Set `PREFILL_BROWSER_MATRIX_REQUIRED=true` to make a missing scenario
URL fail visibly instead of reducing the matrix silently, which is what an
evidence run wants.

### Login started at an external identity provider

Checks that a voter who signs in at an external SAML identity provider lands on the
election list of the Voting Portal without ever opening the portal's own login URL.
The event realm must have that identity provider configured for IdP-initiated SSO.

- `IDP_INITIATED_SSO_URL`: the identity provider URL that starts the login for the event realm
- `IDP_INITIATED_USERNAME` and `IDP_INITIATED_PASSWORD`: a test voter at the identity provider
- `IDP_INITIATED_USERNAME_SELECTOR` and `IDP_INITIATED_PASSWORD_SELECTOR`: the identity
  provider's form fields, when they are not `input[name="username"]` and `input[name="password"]`

Run `yarn test:idp-initiated-login:e2e`. Without the three required variables the test is
reported as skipped; set `IDP_INITIATED_LOGIN_REQUIRED=true` to make that a failure.
