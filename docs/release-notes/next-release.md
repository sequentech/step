<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Next Release

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
1. Go to realm roles click on "Create role"
2. Add the following roles: `events-read` `events-create` `events-edit`
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
