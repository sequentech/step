<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Full admin-portal election order regression

Tracks https://github.com/sequentech/meta/issues/12787.

This browser check uses the production admin portal, Keycloak login, and real
Hasura persistence. It does not intercept requests or substitute form components.
It requires a **disposable** STEP devcontainer installation with the admin portal,
PostgreSQL migrations, Hasura metadata, and backend services configured. Seed an
event with exactly two elections and the configured BASE_LANGUAGE and
ADDED_LANGUAGE available in tenant settings. Use an English
admin interface and an account with election event editing permission.

Install this standalone test's dependencies with `npm install`, and install Google
Chrome in the devcontainer. Supply these environment variables (never commit
credentials or customer exports):

```
PORTAL_URL=http://localhost:3002
HASURA_URL=http://localhost:8080/v1/graphql
HASURA_ADMIN_SECRET=<local development secret>
PORTAL_USERNAME=<test administrator>
PORTAL_PASSWORD=<test password>
EVENT_ID=<fixture event UUID>
ORIGINAL_FIRST_ID=<original first election UUID>
TARGET_FIRST_ID=<election UUID to move first>
ORIGINAL_FIRST_NAME=<unique original first election name>
TARGET_FIRST_NAME=<unique target election name>
BASE_LANGUAGE=es
ADDED_LANGUAGE=en
ALLOW_FIXTURE_RESET=true
```

Run `npm test` in this directory inside the devcontainer. The test resets the
fixture to distinct ranks (original first: 0; target: 1), custom ordering, and
BASE_LANGUAGE only. It verifies every reset and navigates directly to EVENT_ID
through Keycloak login. Other presentation fields are preserved.

It enables ADDED_LANGUAGE and drags the target first. On fixed branches it first
takes the browser offline during Save, checks the network failure and unchanged
database without a success notification, then reconnects and repeats the save. It waits for both positions and event languages, checks that one event
update contains normalized settings and no form-only fields, and verifies order
and languages after a full reload. It does not intercept or replace requests.

For historical reproduction, run against `v10.0.0-rc.33` with
`EXPECT_BROKEN=true`: the original ranks and languages remain after save/reload.
The video used the original export's null ranks; the reusable test deliberately
uses distinct initial ranks to avoid nondeterministic database row ordering.
Never use this switch for fixed-branch CI.

The runtime save fix is already present in main (#3065) and release/10.0 (#3066).
The form must call its parent persistence transform exactly once and await it.
When merging the older #3006 implementation, do not retain both its transform
inside `onSave` and the newer `saveTransform` wrapper.
