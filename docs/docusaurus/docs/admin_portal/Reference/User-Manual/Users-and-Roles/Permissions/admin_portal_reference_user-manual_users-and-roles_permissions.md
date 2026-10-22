---
id: admin_portal_reference_user_manual_users_and_roles_permissions
title: Admin Portal Reference User Manual Users And Roles Permissions
---

Permissions are assigned to roles under **Users and Roles** > **Roles**. A user receives the
permissions of the roles assigned to them.

## Exports and Other Sensitive Actions

An election-event export needs `election-event-read`, which covers the event configuration and its
scheduled events. Each other kind of data it includes needs the permission that exports or reads
that data on its own:

| Included data | Permission |
|---|---|
| Voters | `voter-export` |
| Applications | `application-export` |
| Activity logs | `logs-export` |
| Tally | `tally-results-read` |
| Reports | `report-read` |
| Publications | `publish-export` |
| Bulletin board (keys ceremonies and boards) | `export-ceremony` |
| Documents | `document-download` |

| Action | Required permissions |
|---|---|
| List votes by IP address for the whole election event | `admin-dashboard-view` and `admin-ip-address-view` |
| List votes by IP address for one election | `admin-dashboard-view` and either `admin-ip-address-view` or `election-ip-address-view` |
| Create a role | `role-create` |
| Generate the ballot-images or vote-receipts report | `report-generate` |
| Export activity logs | `logs-export` |
