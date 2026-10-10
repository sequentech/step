---
id: admin_portal_reference_user_manual_users_and_roles_permissions
title: Admin Portal Reference User Manual Users And Roles Permissions
---



This is a placeholder page for the section: Permissions.

Content will be added here soon.

## Trustee and Admin Lockdown Roles

The `trustee` and `admin-lockdown` roles of a new tenant do not include the `admin-user`
permission. Trustees work with the key and tally ceremony permissions, and admin lockdown users
with the publication permissions; the admin portal runs their requests with those permissions.

Tenants created before this change keep the roles they were created with. After upgrading, open
**Users and Roles** > **Roles**, edit the `trustee` and `admin-lockdown` roles of each tenant, and
remove the `admin-user` permission from them. Users must log in again for the change to apply.
