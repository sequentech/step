<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

## Allow restricting Admins to specfic elections
From now on there is an option to restrict access Admin users to specific elections.
A new user attribute called permission_labels was added to the admin portal realm in Keycloak and it's multivalued. 
A new column was added to the election database. 
If there is no permission_label at the election everyone can access it.
If there if permission_label than the permission_labels from the x-hasura-permission_labels mapper from the user attribute needs to include the election permission label.
A new group was added to keycloak called admin-light and a new role and permission in Hasura called permission-label-write. which the new group does not have and can't edit the permission label at the election level and at the user level. 

### Important notes
1. A new user attribute and a new column was added to keycloak and Hasura. 
2. a new Mapper was added (custom Mapper to handle multivalued attribute for Hasura to read it right)
3. A new Permission was added and a new Group to keycloak called admin-light. 

# Next Release

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

## Keycloak: Election event "Events" - add permissions to keycloak realm:

It requires to add a couple of permissions In order use Election event "EVENTS" tab:
1. Go to realm roles, select the admin role and click on "Create role"
2. Add the following roles: `scheduled-event-write`
3. Then Go to "Groups" and choose `admin` group name
4. Go to "role mapping" and click on `Assign role` and add those permissions

## Templates
Handling of templates has been inproved. The templates that are uploaded in the Templates 
section can now be selected at election event level-> Data -> Template.
Therefore the admin can have several templates of the same option, for example: 
MANUALLY_VERIFY_VOTER and later select which one to apply for each election event.
More template types can be added in the future.

It is required to upload the default templates to the S3 public-assets folder:
`step/.devcontainer/minio/public-assets/manual_verification_system.hbs`
`step/.devcontainer/minio/public-assets/manual_verification_user.hbs`
Being the ENV var `PUBLIC_ASSETS_PATH=public-assets` in this case.
The rest of ENV variables starting by `PUBLIC_ASSETS_` that have been removed, used to name 
the files and other assets but they are no longer needed by the system.

## Scheduled Events

There's a new tab "Scheduled Events" in the Election Event section. This lists the
next scheduled events. In order to see this tab, the role `scheduled-event-write`
has to be added in Keycloak and the role has to be added to the `admin` group
for existing tenants. Also notice that this includes a migration to delete the `dates`
column in both Elections and Election Events. There's also a minor speed improvement
in the cast vote action as it's not using hasura graphql calls anymore.
