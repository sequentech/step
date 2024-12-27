<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release 7.0

### Migration to add permissions to keycloak realm

Add the following roles to the sbei group:

```
monitoring-dashboard-view-election-event
monitoring-dashboard-view-election
monitor-authenticated-voters
monitor-all-approve-disapprove-voters
monitor-automatic-approve-disapprove-voters
monitor-manually-approve-disapprove-voters
monitor-enrolled-overseas-voters"
monitor-posts-already-closed-voting
monitor-posts-already-generated-election-results
monitor-posts-already-opened-voting
monitor-posts-already-started-counting-votes
monitor-posts-initialized-the-system
monitor-posts-started-voting
monitor-posts-transmitted-results
monitor-voters-voted-test-election
monitor-voters-who-voted
```

## 🐞 Hasura stops working when assigning too many permissions

In order to reduce the jwt size for tenant realms, we need to remove the
"realm_access" key from the jwt. To do this, go to each tenant realm in
Keycloak, then `Client scopes` > `roles` > `Mappers` > `realm roles` then set
`Add to access token` Off.

## ✨ Windmill > Enrollment: enable fuzzy search

In production environments, ensure that the `unaccent` extension is enabled for
the postgres used by keycloak:

```
CREATE EXTENSION IF NOT EXISTS unaccent;
```

### S3: New files to be uploaded

For existing environments the following files need to be uploaded to S3:

- .devcontainer/minio/public-assets/ov_turnout_per_aboard_and_sex_extra_config.json
- .devcontainer/minio/public-assets/ov_turnout_per_aboard_and_sex.json
- .devcontainer/minio/public-assets/ov_turnout_per_aboard_and_sex_user.hbs
- .devcontainer/minio/public-assets/ov_turnout_per_aboard_and_sex_system.hbs
- .devcontainer/minio/public-assets/ov_with_voting_status_extra_config.json
- .devcontainer/minio/public-assets/ov_with_voting_status.json
- .devcontainer/minio/public-assets/ov_with_voting_status_user.hbs
- .devcontainer/minio/public-assets/ov_with_voting_status_system.hbs
- .devcontainer/minio/public-assets/ovcs_events_system.hbs
- .devcontainer/minio/public-assets/ovcs_events_user.hbs
- .devcontainer/minio/public-assets/ovcs_events.json
- .devcontainer/minio/public-assets/ovcs_statistics_system.hbs
- .devcontainer/minio/public-assets/ovcs_statistics_user.hbs
- .devcontainer/minio/public-assets/ovcs_statistics.json
  
## ✨ Admin Portal > Approvals: Export/Import applicants

In order to be able to Import / Export applications from Admin-portal "APPROVALS" tab,
You need to add this permissions to your tenant in keycloak:

- `application-export`
- `application-import`

1. login to Keycloak
2. Choose your tenant
3. Go to Realm roles and create new role `application-export` and then another role `application-import`.
4. Go to Groups and choose `Admin` group.
5. Go to tab `Role mapping` and `assign role`
6. Add the `application-export` and `application-import` roles.
## ✨ Export trustee config, update offline installation

In production environments, add this new permission: `trustees-export`.
