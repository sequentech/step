<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release NEXT

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

In order to reduce the jwt size for tenant realms, we need to remove the "realm_access" key from the jwt.
To do this, go to each tenant realm in Keycloak, then `Client scopes` > `roles` > `Mappers` > `realm roles`
then set `Add to access token` Off.

## ✨ Windmill > Enrollment: enable fuzzy search

In production environments, ensure that the `unaccent` extension is enabled for the postgres used by keycloak:

```
CREATE EXTENSION IF NOT EXISTS unaccent;
```