<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release 6.0

## ✨ Admin Portal: New `Election Event` > `Reports` section

There's a new tab `Reports` in the Election Event. This consolidates the way
reports are configured throughout for the election event and elections. In order
to see this tab, the role `report-read` has to be added in Keycloak and the role
has to be added to the `admin` group for existing tenants. Also notice that this
includes changes on how reports are configured, with a new `report` table that
configures reports for the whole election event, as previously report
configuration was scattered in different places and this is also a breaking
change that will make previous elections events behave differently.

### Migration to add permissions to keycloak realm

It requires to add a couple of permissions In order use Election event
`Scheduled Events` tab:
1. Go to realm roles, select the admin role and click on `Create role`
2. Add the following roles: `report-read` and `report-write`
3. Then Go to `Groups` and choose `admin` group name
4. Go to `role mapping` and click on `Assign role` and add those permissions


## ✨ Admin Portal: Adding `Help` > `User Manual` in sidebar

In [#1700](https://github.com/sequentech/meta/issues/1700) we have added support
to show in the `Admin Portal` a `Help` item with configure links that open in a
new tab.

This is configurable from `Admin Portal` > `Settings` > `Look & Feel`, in a new
field that appears there called `Help Links`.

You can configure the links with the following format (this is just an example):

```json
[
    {
        "url": "https://example.com",
        "title": "System Manual",
        "i18n": {
            "en": {
                "title": "System Manual"
            },
            "es": {
                "title": "Manual del Sistema"
            },
            "fr": {
                "title": "System Manual"
            },
            "tl": {
                "title": "System Manual"
            },
            "cat": {
                "title": "System Manual"
            }
        }
    }
]
```

By default it will be undefined so it won't show. If the list is an empty array,
it will also not show.

## ✨ Migrate to PostgreSQL backend for the bulletin board

In [#589](https://github.com/sequentech/meta/issues/589) we are
introducing a new service called b3 on the trustees. The braid service
at the trustees used to use ImmuDB for keeping the index, but it will
use the B3 service from now on instead --through a protobuf API--,
which in turn uses PostgreSQL.

In order to create the database and schema that B3 expects at the
PostgreSQL instance, the following script has to be executed,
**manually for now**:

```sql
SELECT 'CREATE DATABASE b3'
    WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'b3')\gexec

\c b3

CREATE TABLE IF NOT EXISTS INDEX (
            id SERIAL PRIMARY KEY,
            board_name VARCHAR UNIQUE,
            is_archived BOOLEAN,
            cfg_id VARCHAR,
            threshold_no INT,
            trustees_no INT,
            last_message_kind VARCHAR,
            last_updated TIMESTAMP,
            message_count INT,
            batch_count INT DEFAULT 0,
            UNIQUE(board_name)
        );
```

### CLI options and environment variables

B3 has the following CLI options:

#### B3 service

PostgreSQL related options:

- `--host`
- `--port`
- `--username`
- `--password`
- `--database`

Generic options:

- `--bind`: Where B3 listens for GRPC connections
- `--blob-root`: optional
- `--max-message-size-bytes`: defaulted

#### Braid

Braid reads the following environment variables --they also correspond
to `braid` binary CLI flags if they are preferred--:

```
# Braid connection to B3
B3_URL=http://b3:50051  # --b3-url CLI option
```

## ✨ Added new permissions

### Permissions added

```
election-data-tab
election-event-areas-tab
election-event-data-tab
election-event-keys-tab
election-event-logs-tab
election-event-publish-tab
election-event-reports-tab
election-event-scheduled-tab
election-event-tally-tab
election-event-tasks-tab
election-event-voters-tab
election-publish-tab
election-voters-tab

contest-create
contest-delete
contest-read
contest-write

candidate-create
candidate-delete
candidate-read
candidate-write

election-create
election-delete
election-read
election-write

election-event-archive
election-event-create
election-event-delete
election-event-read
election-event-write
```

To add the permissions manually in Keycloak the procedure followed is:

1. Go to realm roles, select the admin role and click on `Create role`
2. Add the following roles: `report-read` and `report-write`
3. Then Go to `Groups` and choose `admin` group name
4. Go to `role mapping` and click on `Assign role` and add those permissions

As a result:

- The permissions are added in Keycloak under `Realm roles` inside the tenant
- The roles are attached to the `admin` role in `Groups`

The file `.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json` has been updated with the new permissions, roles, and groups
