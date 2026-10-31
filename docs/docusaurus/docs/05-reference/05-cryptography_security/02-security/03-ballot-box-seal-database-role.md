---
id: ballot_box_seal_database_role
title: Database role for the ballot box seal
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The [ballot box seal](../../../02-election_managers/02-reference/02-election-event/20-ballot-box-seal.md)
locks each sealed ballot box with database triggers. The ballots, the seals and
the electoral log of an election event are in the event's own electoral-log
database (see
[the electoral-log ballot box](../../../07-developers/13-electoral-log/03-electoral-log-ballot-box.md)),
and the triggers that lock a box are there. The settings, the seal policies
and the seals' state for the Admin Portal are in the Hasura database, with
triggers of their own. A trigger holds only against database roles that can
neither disable nor skip it. This page states what each database enforces,
what it can't stop in this release, and how to keep the databases consistent.

## What the event's database enforces

Every election event's electoral-log database gets these triggers from the
electoral-log schema, when the event's database is created and when
`electoral-log-admin upgrade-event-databases` applies the schema again. Each
refusal uses SQLSTATE `42501`.

| Trigger | Table | Refuses |
| --- | --- | --- |
| `ballot_box_seal_guard` | `ballot_box_ballot` (cloned to each event's partition) | Any insert, update or delete of a ballot whose ballot box (the old or new election and area) has a seal, sealed or failed: `ballot_box_sealed`. It also refuses writes from a transaction that is not at READ COMMITTED: `ballot_box_seal_requires_read_committed`. A stricter isolation level could read a snapshot older than the seal. |
| `ballot_box_seal_guard_truncate` | `ballot_box_ballot` and each event's partition | A truncate while the table (or the event) holds a sealed box. |
| `ballot_box_seal_permanence` | `ballot_box_seal` | Any update or delete of a seal: `ballot_box_seal_permanent`. |
| `ballot_box_seal_permanence_truncate` | `ballot_box_seal` | Any truncate. |

A write of a ballot takes the box's seal lock shared before it checks for a
seal; the sealer takes the same lock exclusively, without waiting. So a write
either commits before the sealer reads the box and is in the seal, or waits for
the seal and is refused. The voters' table (`ballot_box_voter`) is changed only
in the same statements as `ballot_box_ballot`, so guarding the ballots guards
the whole box.

A seal and its `BallotBoxSealed` record (or, for a failed seal, its
`BallotBoxSealFailed` record) are written to the event's database in one
transaction: one is never stored without the other. Each box has at most one
seal, and its record has the fixed delivery ID
`ballot-box-seal:<event>:<election>:<area>`, so the board holds at most one
seal record per box.

`electoral-log-admin drop-event-database` refuses a database that holds seals,
and the ballot box of an event with seals can't be dropped on its own.

## What the Hasura database enforces

Migrations `1791000001600_ballot_box_seal`,
`1791000001700_ballot_box_seal_hardening`,
`1791000001800_ballot_box_seal_record_policy` and
`1791000001900_ballot_box_seal_record_guards` add these triggers. Each refusal
uses SQLSTATE `42501`, except an unknown record policy value (`22023`).

| Trigger | Table | Refuses |
| --- | --- | --- |
| `ballot_box_seal_permanence` | `ballot_box_seal` | Deleting a seal row, and any change other than the allowed steps: pending to sealed (with its seal data and its record on the electoral log) or failed; sealed to published, each publication field set once, with a public path exactly when the event's effective Seal Record Publication policy is Public; the sealer's last attempt while pending. |
| `ballot_box_seal_permanence_truncate` | `ballot_box_seal` | Any truncate. |
| `guard_ballot_box_seal_policy_update` | `election_event` | Changing the seal policy once voting has opened. |
| `guard_ballot_box_seal_event_settings` | `election_event` | For a Seal at close event, once voting has opened: changing the effective contest encryption, delegated voting, weighted voting or Seal Record Publication policy (a missing or empty value counts as the default), or the event's bulletin board reference. |
| `guard_ballot_box_seal_record_policy_value` | `election_event` | For a Seal at close event, on every insert or update: a `ballot_box_seal_record_policy` other than `restricted`, `public`, absent or JSON null (SQLSTATE `22023`). |
| `ballot_box_seal_record_document_guard` | `document` | Updating or deleting the document of a published seal's record. |
| `ballot_box_seal_record_document_truncate` | `document` | A truncate while any published seal has a record document. |
| `guard_ballot_box_seal_closed_channels` | `election` | For a Seal at close event: changing the status of a Closed voting channel (`ballot_box_seal_closed_is_final`). |
| `election_seal_guard` | `election` | Deleting an election that has seals, in any status. |

Hasura's `ballot_box_seal` table follows the event database's seals: the close
creates its pending rows in the same transaction as the status change, the
sealer works through them, and the Admin Portal reads them. Hasura exposes it
read-only: no role can insert, update or delete through it.

## What it cannot stop

PostgreSQL lets two kinds of role get past a trigger:

- the **owner** of a table can disable its triggers (`ALTER TABLE … DISABLE
  TRIGGER`);
- a **superuser** can do the same, or skip triggers for a session (for example
  with `session_replication_role = replica`), or change the trigger functions.

**In this release the services can bypass the guard of the event databases.**
The electoral-log application role owns every event database, and Harvest and
windmill connect to the event databases as that role. Whoever holds that role's
credentials, which every Harvest and windmill instance has, can disable
`ballot_box_seal_guard`, change or delete the ballots of a sealed box, and
enable the guard again. The guard stops application paths and mistakes; it
does not stop someone who controls a service or its configuration. A separate
role that may only read and write the rows, while another role owns the
schema, is a planned follow-up.

What remains is **detection**, not prevention:

- **The tally re-verifies every sealed box.** Before counting, it reads the
  box's seal, its signed `BallotBoxSealed` record and every stored ballot of
  the box in one snapshot of the event's database, recomputes the hash of each
  ballot and compares the ballots with the manifest whose hash is in the signed
  record. Any difference stops that election's tally with a
  `TallyBallotBoxRejected` ERROR entry. A ballot added, changed, removed or
  given another status after the seal is caught this way.
- **Checkpoints anchor the log.** The signed electoral-log checkpoints that
  Step publishes to the Hasura database (when voting opens and closes,
  periodically while it is open, and after every results tally) cover the
  `BallotBoxSealed` records. A seal record removed or rewritten afterwards no
  longer matches them, and the electoral-log audit reports it.
- **Observers' copies.** The seal records and seal hashes that observers kept,
  and `step-cli step verify-ballot-box-seal` with `--expected-key` set to the
  event key's fingerprint obtained before the close, show a seal that was
  replaced or signed again.

Keep the application role's credentials as secret as the database superuser's,
log every connection that doesn't come from a service, and alert on any
`ALTER TABLE … DISABLE TRIGGER` in the event databases.

The Hasura database's triggers hold against Hasura, windmill and Harvest only
if they don't connect as the owner of its tables or as a superuser. In
production, connect them as a role that:

- is not a superuser;
- owns no table, function or schema;
- has only the privileges the services need on the tables.

This repository has no deployment configuration that creates such a role: it
belongs to the hosting setup of each deployment. Check it before the first
election that uses **Seal at close**.

:::warning Development and compose setups
The development compose files connect Hasura and windmill to Hasura's database
as the `postgres` superuser. There the guards protect against application
mistakes, not against someone with database access. Do not run a real election
with that setup.
:::

## Check a deployment

In an event's electoral-log database (named after the base database and the
event's ID without hyphens):

```sql
-- Are the guards enabled? ('O' means enabled; ballot_box_seal_guard shows on the
-- partitioned table and on each event's partition.)
SELECT tgrelid::regclass AS table_name, tgname, tgenabled
FROM pg_trigger
WHERE tgname IN (
  'ballot_box_seal_guard', 'ballot_box_seal_guard_truncate',
  'ballot_box_seal_permanence', 'ballot_box_seal_permanence_truncate'
)
ORDER BY 1, 2;

-- The seals the database holds.
SELECT election_id, area_id, status, log_delivery_id, log_leaf_index, created_at
FROM ballot_box_seal
ORDER BY election_id, area_id;
```

In the Hasura database, as an administrator: the service role should show
`rolsuper = false` and own none of the tables.

```sql
SELECT rolname, rolsuper, rolcreaterole, rolbypassrls
FROM pg_roles
WHERE rolname = '<service role>';

SELECT tablename, tableowner
FROM pg_tables
WHERE schemaname = 'sequent_backend'
  AND tablename IN ('ballot_box_seal', 'election_event', 'election', 'document', 'secret');

-- ('O' means enabled; expect 6 rows)
SELECT tgname, tgenabled
FROM pg_trigger
WHERE tgname IN (
  'ballot_box_seal_permanence', 'ballot_box_seal_permanence_truncate',
  'guard_ballot_box_seal_policy_update', 'guard_ballot_box_seal_event_settings',
  'guard_ballot_box_seal_closed_channels', 'election_seal_guard'
);
```

## Services needed at every close

Before this feature, the tally workers, the Keycloak census and the public
bucket were needed only at tally time. With Seal at close, every close needs:

- the Beat scheduler, and windmill workers that consume `short_queue`, with free
  capacity during close windows;
- the ballot box sequencer: a box is sealed only once every ballot it accepted
  is on the electoral log;
- the event's electoral-log database;
- the Keycloak database;
- the key store (see below) and its master secret;
- the documents buckets.

When you upgrade a deployment and enable Seal at close, check that its worker
deployment keeps `short_queue` consumers running at every close and sizes them
for the largest close. See the runbook's
[What must run at every close](../../../02-election_managers/01-tutorials/22-admin_portal_tutorials_ballot-box-seal-runbook.md#what-must-run-at-every-close).

### Upgrade notes

- **Event databases created before the seal schema** get the seal table and
  the guards from `electoral-log-admin upgrade-event-databases`. Run it before
  the first close of an event that seals.
- **Migration `1791000001700`** replaces triggers on busy tables and waits at
  most 10 seconds for their locks. If it times out, Hasura's startup migration
  fails and Hasura restarts in a loop until the blocking transaction ends.
  Apply it in a quiet window, not during voting.
- **Elections started before voting period dates were recorded** read as
  never having run. For them, no grace period is added to the seal deadline,
  and a scheduled close doesn't close their named never-started channels.
- **The settings lock** compares effective values: a missing or empty contest
  encryption, delegated or weighted voting setting counts as its default, so
  existing events that never stored them keep saving normally.

## Where the signing key is kept

The seal is signed by the election event's protocol manager key. The key is
stored in the Hasura database's secret table (`sequent_backend.secret`), keyed
by the tenant, the event and the board, and encrypted under the deployment's
master secret. By default the master secret is configured on the windmill
services. Anyone who holds both the database and the master secret can sign as
the event.

Nothing outside the database anchors this key today (no HSM, and the trustees'
signed configuration holds the election boards' keys, not the event board's).
Moving it out of the database is out of scope of this feature.

## What a superuser can still do

Besides bypassing the guard (see above), someone with superuser or owner rights
on both databases can also change what the tally checks against:

1. disable the guards, change a sealed box's ballots, delete its seal from the
   event's database, and replace the Hasura seal row with a new pending one;
2. point the event's bulletin board reference at another board, and put that
   board's key in the event's place in the secret table;
3. the sealer then seals the changed box again on the other board, and
   overwrites the seal record (public or restricted). The tally reads that
   board and key, finds one valid seal, and counts the changed box.

The triggers on the seals and on the bulletin board reference make these more
triggers to bypass, not impossible. What still catches it:

- the published **checkpoints** of the original board, which cover its
  original `BallotBoxSealed` record;
- the **copies** of the seal records and seal hashes that observers kept;
- an auditor running `step-cli step verify-ballot-box-seal` with
  `--expected-key` set to the event key's fingerprint **obtained before** the
  change: the new seal is signed by another key.

Other evidence:

- **The audit trail.** PostgreSQL's audit log records writes (`pgaudit.log =
  'WRITE'`).
- **The seal record no longer matches** the copies observers kept.

Detection is not prevention. Keeping the services on least-privilege roles,
and the superuser and owner logins apart and alerted, is what prevents it.

## Keep the databases consistent

A seal and its record on the electoral log are written to the event's database
in one transaction, so restoring that database never separates them. The
Hasura row follows in a second step:

- **The Hasura database loses the seal** (an asynchronous replica is promoted,
  or a point-in-time restore) while the event's database keeps it: the row is
  pending again, and the sealer's next attempt finds the stored seal and copies
  it into the row. Nothing is sealed twice.
- **The event's database is restored to before the seal** while Hasura keeps
  it: the box has no seal there, and its guard no longer refuses writes. Casts
  stay refused (the cast check reads the seal deadline from Hasura), but the
  tally rejects the box ("the ballot box's seal row says it is sealed, but the
  stored seal is missing"), and the published checkpoints no longer match the
  board. This is an incident: report it, and restore the event's database to
  after the seal.

A common close seals every box at the same moment, so one such event can hit
all of them. So:

- back up and restore the Hasura database and the event databases to
  consistent points, never one without the other;
- during close and seal windows, use synchronous replication, or a failover
  that loses no committed transactions;
- don't plan failovers or restores during close windows;
- before treating a failed seal or a rejected box as tampering, check the
  recovery history (see the
  [runbook](../../../02-election_managers/01-tutorials/22-admin_portal_tutorials_ballot-box-seal-runbook.md#after-a-restore-or-failover)).
