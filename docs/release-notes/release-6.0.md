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

### Environment variables

B3 has the following environment variables--they also correspond to
`b3` binary CLI flags if they are preferred--:

#### B3 service

```
# B3 connection to PostgreSQL envvars
B3_PG_HOST=postgres-b3
B3_PG_PORT=5432
B3_PG_USER=postgres
B3_PG_PASSWORD=postgrespassword
B3_PG_DATABASE=b3
# Where B3 listens for GRPC connections
B3_BIND=0.0.0.0:50051
```

#### Braid

Braid has the following environment variables --they also correspond
to `braid` binary CLI flags if they are preferred--:

```
# Braid connection to B3
B3_URL=http://b3:50051
```
