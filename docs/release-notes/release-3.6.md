<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release 3.6

## Allow to delete an election event

From now on there's a new permission to be able to delete an election event.
Existing deployments need to adjust to this change by adding this permission to
the admin role.

### Steps to Manually Add `election-event-delete` to `admin` group

1. **Login to the Keycloak Admin Console**

   - Open the Keycloak Admin Console in your browser.
   - Log in using your admin credentials.
   - Choose the Realm to be the admin-portal default realm

2. **Create the `election-event-delete` role**

   - Navigate to the `Realm Roles` section under the Admin realm.
   - Click the `Create role` blue button.
   - In the Create Role form, fill the `Role Name` with the value
     `election-event-delete`.
   - Click `Save`.

3. **Add the `election-event-delete` role to the `admin` group**

   - Navigate to the `Groups` section under the Admin realm.
   - Click in the blue `admin` group name in the right side.
   - In the `Group Details` sidebar at the right, click in the `Role Mapping`
     tab.
   - Click the `Assign Role` blue button.
   - In the dialog that appears, click the checkbox for `election-event-delete`
     item.
   - Click the `Assign` button.

## Allow to assign a user to a trustee

From now on, each admin user can be assigned a trustee. Existing deployments
need to adjust to this change by assigning the right trustee to the admin users
that have the trustee role.

### Steps to Manually Assign a Users to a Trustee

1. **Login to the Keycloak Admin Console**

   - Open the Keycloak Admin Console in your browser.
   - Log in using your admin credentials.
   - Choose the Realm to be the admin-portal default realm

2. **Assign `trustee1` to user `trustee1`**

   - Navigate to the `Users and Roles` section under your realm.
   - Click on `...` at the right of user `trustee1`.
   - On the menu that appears, click on `Edit`.
   - In the dialog that appears, set the `Act as a Trustee` field to `trustee1`.
   - Click on `Save`.
   - Repeat the process for `trustee2`.

## Send keycloak logs to harvest/immudb

For keycloak to be able to send the logs, these variables need to be configured
in the keycloak container:

- KEYCLOAK_URL
- KEYCLOAK_CLIENT_ID
- KEYCLOAK_CLIENT_SECRET
- HARVEST_DOMAIN

For example in production deployments, the values for these variables are:

- name: KEYCLOAK_URL
 value: http://keycloak-keycloakx-http/auth
- name: HARVEST_DOMAIN
 value: "harvest:8400"

Also, the `service-account` client for the default tenant realm `tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5` has to be configured:

- Go to `Clients`, then `service-account`.
- Click on `Client scopes`, then `service-account-dedicated`.
- If `x-hasura-tenant-id` exists and the type is not `Hardcoded claim`, then click on the three dots `...` and `Delete` to delete it.
- Then click `Add mapper` > `By configuration` > `Hardcoded claim`.
- Configure:
  - Name: `x-hasura-tenant-id`.
  - Token Claim Name: `https://hasura\.io/jwt/claims.x-hasura-tenant-id`.
  - Claim value: `90505c8a-23a9-4cdf-a26b-4e19f6a097d5`.
- Click `Save`.
