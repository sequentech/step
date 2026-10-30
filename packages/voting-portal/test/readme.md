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
- get email verification otp from keycloak logs
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
