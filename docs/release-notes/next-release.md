<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release NEXT


## ✨ Admin Portal > Ip Addresses Widget at Election level

### Added new permissions for IP Address view in dashboard

To add the permissions manually in Keycloak the procedure followed is:

1. Go to realm roles, select the admin role and click on `Create role`
2. Add all the roles in the list
3. Then Go to `Groups` and choose `admin` group name
4. Go to `role mapping` and click on `Assign role` and add those permissions

The list of new permissions is:

```
admin-ip-address-view
```

As a result:

- The permissions are added in Keycloak under `Realm roles` inside the tenant
- The roles are attached to the `admin` role in `Groups`

The file `.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json` has been updated with the new permissions, roles, and groups
    
## ✨ Admin Portal > Voters > required fields when create/edit a user

    ### User / Voter edit or create required fields

    - to define a field as required in users or voters screens we must go to keycloak profiles attributes:
        - for the users, at tenant level
        - for voters, at a election event level

    - the attribute can be marked as:
        - not required
        - required. In that case the are this levels of control:
            - admin, means required in the admin portal to a keycloak admin
            - user, means required for an admin portal admin
            - both

    - in FE, the field for the attribute is 
        - always required **if the keycloak attribute is required** no matter if there is defined for a user or an admin.  
        - not required **if the keycloak attribute is not required**

