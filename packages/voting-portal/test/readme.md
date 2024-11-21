<!--
SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>

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
