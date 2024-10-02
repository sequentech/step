<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Next Release

## ✨ Keycloak: Add system info using env vars in header

[This feature](https://github.com/sequentech/meta/issues/1699) adds the system
info using env vars in the header. It's applied in keycloak, and then also in
admin-portal, voting-portal and ballot-verifier.

### Migration of custom logo in Keycloak

It re-arranges the elements and styles in the header in keycloak, so any custom
styles applied to especially changing the logo in keycloak will need a
migration. Now instead of applying the logo css should be applied to the
`#kc-header-wrapper div.logo` element instead of directly to
`#kc-header-wrapper` as it was typically done before.

### Migration of environment variables

It requires a couple of environment variables set to keycloak container that
will be used to display system version and hash: `APP_VERSION` and `APP_HASH`.

Similarly, the same new env vars `APP_VERSION` and `APP_HASH` are now also used
in global settings by admin-portal and voting-portal (ballot-verifier is still
missing having its own global-settings file).