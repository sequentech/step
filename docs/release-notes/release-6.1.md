<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release 6.1

## ✨ Added new permissions to tree menu

To add the permissions manually in Keycloak the procedure followed is:

1. Go to realm roles, select the admin role and click on `Create role`
2. Add all the roles in the list
3. Then Go to `Groups` and choose `admin` group name
4. Go to `role mapping` and click on `Assign role` and add those permissions

The list of new permissions is:

```
users-menu
settings-menu
templates-menu
settings-election-types-tab
settings-voting-channels-tab
settings-templates-tab
settings-languages-tab
settings-localization-tab
settings-look-feel-tab
settings-trustees-tab
settings-countries-tab

```

As a result:

- The permissions are added in Keycloak under `Realm roles` inside the tenant
- The roles are attached to the `admin` role in `Groups`

The file `.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json` has been updated with the new permissions, roles, and groups

## ✨ Add Permissions for Buttons and Tabs

### Added new permissions for Election Event Voters

To add the permissions manually in Keycloak the procedure followed is:

1. Go to realm roles, select the admin role and click on `Create role`
2. Add all the roles in the list
3. Then Go to `Groups` and choose `admin` group name
4. Go to `role mapping` and click on `Assign role` and add those permissions

The list of new permissions is:

```
voter-import
ee-voters-columns
voter-manually-verify
ee-voters-logs
voter-export
ee-voters-filters
voter-delete
voter-change-password

```

As a result:

- The permissions are added in Keycloak under `Realm roles` inside the tenant
- The roles are attached to the `admin` role in `Groups`

The file `.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json` has been updated with the new permissions, roles, and groups
