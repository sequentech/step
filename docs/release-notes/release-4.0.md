<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release 4.0

## ✨ Support limiting IP by country

[This feature](https://github.com/sequentech/meta/issues/1696) allows limit
voting depending on the IP address of the country of registration or voting can
be done from anywhere in the world.

### Env vars changes

There are multiple environment variable changes introduces in this PR:
- `KEYCLOAK_PUBLIC_URL`, which contains the base URL of keycloak and thus the
  one that needs to be blocked when using this feature. This is added to both
  `harvest` and `windmill` services.
- `VOTING_PORTAL_URL` is also the base URL for the public Voting Portal and is
  also used to be blocked by country when using this feature. This variable was
  used before in `windmill` service, but now also added to the `harvest`
  service.

## ✨ Keycloak: Add system info using env vars in header

[This feature](https://github.com/sequentech/meta/issues/1699) adds the system
info using env vars in the header. It's applied in keycloak, and then also in
admin-portal, voting-portal and ballot-verifier.

### Migration of custom logo css in Keycloak

It re-arranges the elements and styles in the header in keycloak, so any custom
styles applied to especially changing the logo in keycloak will need a
migration. Now instead of applying the logo css should be applied to the
`#kc-header-wrapper div.logo` element instead of directly to
`#kc-header-wrapper` as it was typically done before.

### Deployment: Showing Version and Hash using env vars and `global-settings.json`

It requires a couple of environment variables set to keycloak container that
will be used to display system version and hash: `APP_VERSION` and `APP_HASH`.

Similarly, the same new variables `APP_VERSION` and `APP_HASH` are now also used
in `global-settings.json` by admin-portal, voting-portal and ballot-verifier.
This requires a change in deployment scripts to obtain the hash, otherwise a
default `-` value will be shown.

## Keycloak: Election Event `Scheduled Events` feature

### Scheduled Events

There's a new tab `Scheduled Events` in the Election Event section. This lists
the next scheduled events. In order to see this tab, the role
`scheduled-event-write` has to be added in Keycloak and the role has to be added
to the `admin` group for existing tenants. Also notice that this includes a
migration to delete the `dates` column in both Elections and Election Events.
There's also a minor speed improvement in the cast vote action as it's not using
hasura graphql calls anymore.

### Migration to add permissions to keycloak realm

It requires to add a couple of permissions In order use Election event
`Scheduled Events` tab:
1. Go to realm roles, select the admin role and click on `Create role`
2. Add the following roles: `scheduled-event-write`
3. Then Go to `Groups` and choose `admin` group name
4. Go to `role mapping` and click on `Assign role` and add those permissions

### Migration of scheduled events configuration

Scheduled Events like `Start Voting Period` and `End Voting Period` have been
moved from `Election` and `Election Event` to the `Scheduled Events` tab. They
have been renamed, because before the event processor was `START_ELECTION`
instead of `START_VOTING_PERIOD`, and has suffered changes in the table
structure.

This means that any previous election with scheduled events created or
configured in a release previous to this one will not work properly. These
elections should be deleted.

### Migrations of election events import/export json

Inside the election event json import format, Scheduled Events like `Start
Voting Period` and `End Voting Period` have been moved from inside the
`election` and `election_event` to the `scheduled_events` section.

This means that any previous election event exported in a previous version will
not work properly. These configs should be adapted or re-exported and
re-configured to work properly.

## Improved Templates section

Handling of templates has been improved. The templates that are uploaded in the
Templates section can now be selected at `Election Event` -> `Data` tab ->
`Template`. Therefore the admin can have several templates of the same option,
for example: `MANUALLY_VERIFY_VOTER` and later select which one to apply for
each election event. More template types can be added in the future.

It is required to upload the default templates to the S3 public-assets folder:
`step/.devcontainer/minio/public-assets/manual_verification_system.hbs`
`step/.devcontainer/minio/public-assets/manual_verification_user.hbs` Being the
ENV var `PUBLIC_ASSETS_PATH=public-assets` in this case. The rest of ENV
variables starting by `PUBLIC_ASSETS_` that have been removed, used to name the
files and other assets but they are no longer needed by the system.
