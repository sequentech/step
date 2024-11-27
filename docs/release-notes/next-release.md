<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release NEXT

## ✨ Add Permissions for Buttons and Tabs

### Added new permissions for Election Event Localization

To add the permissions manually in Keycloak the procedure followed is:

1. Go to realm roles, select the admin role and click on `Create role`
2. Add all the roles in the list
3. Then Go to `Groups` and choose `admin` group name
4. Go to `role mapping` and click on `Assign role` and add those permissions

The list of new permissions is:

```
election-event-localization-selector
localization-create
localization-read
localization-write
localization-delete
```

As a result:

- The permissions are added in Keycloak under `Realm roles` inside the tenant
- The roles are attached to the `admin` role in `Groups`

The file `.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json` has been updated with the new permissions, roles, and groups

### Added new permissions for Election Event Areas

To add the permissions manually in Keycloak the procedure followed is:

1. Go to realm roles, select the admin role and click on `Create role`
2. Add all the roles in the list
3. Then Go to `Groups` and choose `admin` group name
4. Go to `role mapping` and click on `Assign role` and add those permissions

The list of new permissions is:

```
area-create
area-delete
area-export
area-import
area-upsert
election-event-areas-columns
election-event-areas-filters
```

As a result:

- The permissions are added in Keycloak under `Realm roles` inside the tenant
- The roles are attached to the `admin` role in `Groups`

The file `.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json` has been updated with the new permissions, roles, and groups

### Added new permissions for Election Event Tasks

To add the permissions manually in Keycloak the procedure followed is:

1. Go to realm roles, select the admin role and click on `Create role`
2. Add all the roles in the list
3. Then Go to `Groups` and choose `admin` group name
4. Go to `role mapping` and click on `Assign role` and add those permissions

The list of new permissions is:

```
election-event-tasks-back-button
election-event-tasks-columns
election-event-tasks-filters
task-export
```

As a result:

- The permissions are added in Keycloak under `Realm roles` inside the tenant
- The roles are attached to the `admin` role in `Groups`

The file `.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json` has been updated with the new permissions, roles, and groups

### Added new permissions for Election Event Logs

To add the permissions manually in Keycloak the procedure followed is:

1. Go to realm roles, select the admin role and click on `Create role`
2. Add all the roles in the list
3. Then Go to `Groups` and choose `admin` group name
4. Go to `role mapping` and click on `Assign role` and add those permissions

The list of new permissions is:

```
logs-export,
election-event-logs-columns,
election-events-logs-filters
```

As a result:

- The permissions are added in Keycloak under `Realm roles` inside the tenant
- The roles are attached to the `admin` role in `Groups`

The file `.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json` has been updated with the new permissions, roles, and groups

