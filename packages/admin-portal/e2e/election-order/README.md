# Full admin-portal election order regression

Tracks https://github.com/sequentech/meta/issues/12787.

This browser check uses the production admin portal, Keycloak login, and real
Hasura persistence. It does not intercept requests or substitute form components.
It requires a **disposable** STEP devcontainer installation with the admin portal,
PostgreSQL migrations, Hasura metadata, and backend services configured. Seed an
event with exactly two elections, custom election ordering, and names whose
initial alphabetical ordering places ORIGINAL_FIRST_NAME first. Use an English
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
ALLOW_FIXTURE_RESET=true
```

Run `npm test` in this directory inside the devcontainer. The test resets only
the fixture elections' `presentation.sort_order` values to null, then performs a
real drag and save. It checks stored positions 0 and 1 through Hasura and verifies
the order after a full page reload. Other presentation fields are preserved.
The selected event must be the one the test administrator opens after login.

For historical reproduction, run against `v10.0.0-rc.33` with
`EXPECT_BROKEN=true`: the same drag is visible before save, but stored positions
remain null and the original order returns on reload. Never use this switch for
fixed-branch CI.

The runtime save fix is already present in main (#3065) and release/10.0 (#3066).
The form must call its parent persistence transform exactly once and await it.
When merging the older #3006 implementation, do not retain both its transform
inside `onSave` and the newer `saveTransform` wrapper.
