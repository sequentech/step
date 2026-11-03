---
id: auditor_verify-tenant-log-export
title: Verify a tenant log export
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The tenant log records what an organization's administrators do outside its election events:
sign-ins to the Admin Portal, staff accounts and roles, settings, and creating and deleting election
events. This guide shows how to export it as SQL, load it into your own PostgreSQL database and check
the signatures of every entry. See
[Settings > Logs](../../02-election_managers/02-reference/user-manual/settings/settings_logs.md) for
what the log holds.

The same steps work for the SQL export of an election event's Logs tab, which has the same tables.

You need:

- the export file, or an account with the `tenant-logs-export` permission to make it;
- a PostgreSQL server of your own and the `psql` client;
- a tool that verifies Ed25519 signatures. The platform does not ship one for exports; the example
  below uses Python with the `cryptography` and `psycopg` packages.

## 1. Export the log as SQL

1. In the Admin Portal, open **Settings** > **Logs** and select **Export**.
2. Choose the **Scope** (**The whole tenant log**, or **One account**), the range (**From**, **To**)
   if you want one, and the **Timezone**.
3. Choose the **SQL** format and select **Export**.
4. When the **Export Tenant Logs** task completes, download the file. It is named
   `tenant-logs-<date and time in UTC>.sql`.

The export is recorded in the tenant log as a `LogsExported` entry that holds the SHA-256 of the
file. Compute the hash of your copy and compare it with the **SHA-256 of the file** shown in that
entry's popup:

```bash
sha256sum tenant-logs-20280203T040506Z.sql
```

## 2. Load it into PostgreSQL

The file is one transaction that creates two tables and fills them, so load it into a database that
does not have them yet:

```bash
createdb tenant_log_audit
psql -d tenant_log_audit -v ON_ERROR_STOP=1 -f tenant-logs-20280203T040506Z.sql
```

| Table | Content |
|---|---|
| `audit_log` | One row per entry. Its primary key is `(log, id)`. |
| `audit_log_export` | One row whose `manifest` column (JSON) says what the export holds. |

The columns of `audit_log`:

| Column | Type | Content |
|---|---|---|
| `log` | text | `tenant`, or `event <election event ID>` for a row that comes from an election event's log |
| `id` | bigint | The entry's number in its log |
| `created` | bigint | When the entry was written to the log, in seconds since 1970 (UTC) |
| `statement_timestamp` | bigint | When the action happened, in seconds since 1970 (UTC) |
| `statement_kind`, `event_type`, `log_type` | text | The kind of entry, `USER` or `SYSTEM`, and `INFO` or `ERROR` |
| `user_id`, `username` | text | The account the entry is about, when it has one |
| `description` | text | One line that says what happened |
| `message` | jsonb | The whole entry in readable form, with its statement, signer and signatures |
| `statement` | bytea | The exact bytes that were signed |
| `sender_name` | text | The name of the person whose key signed the entry; empty for an entry signed by the log's system key alone |
| `sender_public_key` | text | The signer's public key |
| `sender_signature`, `system_signature` | bytea | The two signatures of `statement` |
| `created_utc` | timestamptz | `created` as a UTC time |
| `created_local`, `time_zone` | text | `created` in the export's timezone, and that timezone's name |

The manifest:

```json
{
  "format": "SQL",
  "scope": "TENANT_LOG",
  "username": null,
  "election_id": null,
  "area_id": null,
  "created_from": null,
  "created_to": null,
  "time_zone": "UTC",
  "entries": 1234,
  "logs": [
    {"log": "tenant", "system_public_key": "<base64>", "entries": 1234}
  ]
}
```

`logs` has one element per log the export read, each with that log's system public key and the number
of its entries in the file. An export of one account with its entries in every election event has
one element for the tenant log and one for each election event, deleted ones included.

A deleted election event's log is named `retired event <event ID>`, and its `system_public_key` is
`null`: the event's keys were deleted with it. The tenant log's entry of the event's deletion keeps
the public key, as `system_public_key` in its details; set it in the manifest's element before
verifying (section 4).

## 3. Check what was loaded

Compare the number of rows with the manifest, overall and for each log:

```sql
SELECT (SELECT count(*) FROM audit_log) AS loaded,
       (manifest ->> 'entries')::bigint AS expected
FROM audit_log_export;

SELECT l.log, l.entries AS expected, count(a.id) AS loaded
FROM audit_log_export,
     jsonb_to_recordset(manifest -> 'logs') AS l(log text, entries bigint)
LEFT JOIN audit_log a ON a.log = l.log
GROUP BY l.log, l.entries;
```

If the export has a range, every row's `created_utc` falls between `created_from` and `created_to`,
both ends included, to the minute.

## 4. Verify the signatures

Every entry carries two Ed25519 signatures over the same message, the bytes in `statement`:

| Signature | Verify with |
|---|---|
| `sender_signature` | The row's `sender_public_key` |
| `system_signature` | The `system_public_key` of the manifest's element whose `log` equals the row's `log` |

Encodings:

- A public key is the base64 (standard alphabet, with padding) of its 32 raw bytes.
- A signature is 64 raw bytes.
- The message is `statement` as stored, with nothing added and no hashing beforehand.

An entry of a change an administrator made (`sender_name` is not empty) is signed with that person's
admin key and with the log's system key. Any other entry, sign-ins included, is signed twice with the
log's system key, so its `sender_public_key` equals the log's `system_public_key`.

The example below verifies every row and prints the ones that fail. Pass it the connection string of
the database you loaded:

```python
import base64
import sys

import psycopg
from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey


def public_key(encoded):
    return Ed25519PublicKey.from_public_bytes(base64.b64decode(encoded))


with psycopg.connect(sys.argv[1]) as db:
    manifest = db.execute("SELECT manifest FROM audit_log_export").fetchone()[0]
    # A deleted event's key is not in the manifest: see section 2.
    system_keys = {
        log["log"]: public_key(log["system_public_key"])
        for log in manifest["logs"]
        if log["system_public_key"]
    }
    rows = db.execute(
        "SELECT log, id, statement, sender_public_key, sender_signature, system_signature"
        " FROM audit_log ORDER BY log, id"
    )
    checked = failed = 0
    for log, entry, statement, sender_key, sender_signature, system_signature in rows:
        checked += 1
        signatures = (
            ("sender", public_key(sender_key), sender_signature),
            ("system", system_keys.get(log), system_signature),
        )
        for name, key, signature in signatures:
            if key is None:
                failed += 1
                print(f"{log} {entry}: no {name} key to verify with")
                continue
            try:
                key.verify(bytes(signature), bytes(statement))
            except InvalidSignature:
                failed += 1
                print(f"{log} {entry}: the {name} signature does not verify")
    print(f"{checked} entries checked, {failed} signatures failed")
    sys.exit(1 if failed else 0)
```

```bash
python3 verify_export.py "dbname=tenant_log_audit"
```

## 5. Check who signed

A valid `sender_signature` shows that the entry was signed with `sender_public_key`. To tie that key
to a person, find the person's `AdminPublicKey` entry in the tenant log: the platform writes one the
first time it creates that administrator's signing key. Its popup shows the key as **Published admin
key**, in the same base64 form as `sender_public_key`.

```sql
SELECT DISTINCT sender_name, sender_public_key
FROM audit_log
WHERE sender_name <> ''
ORDER BY sender_name;
```

Each name should appear with one key, and that key should be the one published for that person.

## What this does and does not show

- A row whose two signatures verify was signed with those keys and its `statement` has not been
  changed since.
- The signatures cover `statement` only. The other columns, `message` included, are copies for
  reading and filtering. `statement` is the Borsh encoding of the statement shown under
  `message -> 'statement'`; its layout is defined by the `Statement` type in
  `packages/electoral-log/src/messages/statement.rs`.
- The signing keys are held by the platform's servers, not by the administrators' own devices. A
  signature shows that the platform signed the entry for that person's session.
- Signatures do not show that the export is complete: a missing row leaves the others valid.
- The system public key in the manifest comes from the same platform as the entries. Keep the keys
  from an earlier export and compare them with later ones.
