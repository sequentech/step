---
id: ballot_box_seal_database_role
title: Database role for the ballot box seal
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The [ballot box seal](../../../02-election_managers/02-reference/02-election-event/20-ballot-box-seal.md)
locks each sealed ballot box with database triggers. Those triggers hold only
against database roles that can neither disable nor skip them. This page states
the deployment prerequisites, what a database superuser can still do, and how
to keep the database and the electoral log consistent.

## What the database enforces

Migrations `1791000001600_ballot_box_seal`,
`1791000001700_ballot_box_seal_hardening`,
`1791000001800_ballot_box_seal_record_policy` and
`1791000001900_ballot_box_seal_record_guards` add these triggers. Each refusal
uses SQLSTATE `42501`, except an unknown record policy value (`22023`).

| Trigger | Table | Refuses |
| --- | --- | --- |
| `cast_vote_seal_guard` | `cast_vote` | Any insert, update or delete of a cast ballot whose ballot box (old or new election and area) has a seal that is not pending: `ballot_box_sealed`. For an event with Seal at close, it also refuses writes from a transaction that is not at READ COMMITTED: `ballot_box_seal_requires_read_committed`. A stricter isolation level could read a snapshot older than the seal. On events with Do not seal it takes no lock. |
| `cast_vote_seal_guard_truncate` | `cast_vote` | A truncate once any box is sealed. |
| `ballot_box_seal_permanence` | `ballot_box_seal` | Deleting a seal, and any change other than the allowed steps: pending to sealed or failed; sealed to published, each publication field set once, with a public path exactly when the event's effective Seal Record Publication policy is Public; the sealer's last attempt while pending; the failure entry's posting time, once. |
| `ballot_box_seal_permanence_truncate` | `ballot_box_seal` | Any truncate. |
| `guard_ballot_box_seal_policy_update` | `election_event` | Changing the seal policy once voting has opened. |
| `guard_ballot_box_seal_event_settings` | `election_event` | For a Seal at close event, once voting has opened: changing the effective contest encryption, delegated voting, weighted voting or Seal Record Publication policy (a missing or empty value counts as the default), or the event's bulletin board reference. |
| `guard_ballot_box_seal_record_policy_value` | `election_event` | For a Seal at close event, on every insert or update: a `ballot_box_seal_record_policy` other than `restricted`, `public`, absent or JSON null (SQLSTATE `22023`). |
| `ballot_box_seal_record_document_guard` | `document` | Updating or deleting the document of a published seal's record. |
| `ballot_box_seal_record_document_truncate` | `document` | A truncate while any published seal has a record document. |
| `guard_ballot_box_seal_closed_channels` | `election` | For a Seal at close event: changing the status of a Closed voting channel (`ballot_box_seal_closed_is_final`). |
| `election_seal_guard` | `election` | Deleting an election that has seals, in any status. |

These apply to every role that connects normally: Hasura (all its roles,
including admin-user), windmill, Harvest and direct SQL. Hasura exposes the
seal table read-only: no role can insert, update or delete through it.

## What it cannot stop

PostgreSQL lets two kinds of role get past a trigger:

- the **owner** of a table can disable its triggers (`ALTER TABLE … DISABLE
  TRIGGER`);
- a **superuser** can do the same, or skip triggers for a session (for example
  with `session_replication_role = replica`), or change the trigger functions.

So the services must not connect as either.

**Prerequisite.** In production, Hasura, windmill and Harvest connect to the
database as a role that:

- is not a superuser;
- owns no table, function or schema;
- has only the privileges the services need on the tables.

Migrations run as a separate owner role. The owner and superuser logins are held
apart from the services and from day-to-day operators, every login is logged,
and their use raises an alert.

This repository has no deployment configuration that creates such a role: it
belongs to the hosting setup of each deployment. Check it before the first
election that uses **Seal at close**.

:::warning Development and compose setups
The development compose files connect Hasura and windmill as the `postgres`
superuser. There the guard protects against application mistakes, not against
someone with database access. Do not run a real election with that setup.
:::

## Check a deployment

Run these as an administrator. The service role should show `rolsuper = false`
and own none of the tables.

```sql
-- Which role do the services use, and is it a superuser?
SELECT rolname, rolsuper, rolcreaterole, rolbypassrls
FROM pg_roles
WHERE rolname = '<service role>';

-- Who owns the protected tables?
SELECT tablename, tableowner
FROM pg_tables
WHERE schemaname = 'sequent_backend'
  AND tablename IN ('cast_vote', 'ballot_box_seal', 'election_event', 'election', 'secret');

-- Are the triggers enabled? ('O' means enabled; expect 8 rows)
SELECT tgname, tgenabled
FROM pg_trigger
WHERE tgname IN (
  'cast_vote_seal_guard', 'cast_vote_seal_guard_truncate',
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
- the Keycloak database;
- the key store (see below) and its master secret;
- immudb;
- the public documents bucket.

When you upgrade a deployment and enable Seal at close, check that its worker
deployment keeps `short_queue` consumers running at every close and sizes them
for the largest close. See the runbook's
[What must run at every close](../../../02-election_managers/01-tutorials/22-admin_portal_tutorials_ballot-box-seal-runbook.md#what-must-run-at-every-close).

### Upgrade notes

- **Migration `1791000001700`** replaces triggers on `cast_vote` and other busy
  tables and waits at most 10 seconds for their locks. If it times out,
  Hasura's startup migration fails and Hasura restarts in a loop until the
  blocking transaction ends. Apply it in a quiet window, not during voting.
- **Seals made before that migration** have no stored names, so their records
  use the names at publication. After a rename, the verify command reports
  such a genuine seal as not valid on its Names line. The auditor guide
  explains
  [how to tell](../../../04-election_auditors/02-tutorials/06-auditor_verify-ballot-box-seal.md#seals-made-before-the-upgrade).
- **Elections started before voting period dates were recorded** read as
  never having run. For them, no grace period is added to the seal deadline,
  and a scheduled close doesn't close their named never-started channels.
- **The settings lock** compares effective values: a missing or empty contest
  encryption, delegated or weighted voting setting counts as its default, so
  existing events that never stored them keep saving normally.

## Where the signing key is kept

The seal is signed by the election event's protocol manager key. The key is
stored in the database's secret table (`sequent_backend.secret`), keyed by the
tenant, the event and the board, and encrypted under the deployment's master
secret. By default the master secret is configured on the windmill services.
Anyone who holds both the database and the master secret can sign as the
event.

Nothing outside the database anchors this key today (no HSM, and the trustees'
signed configuration holds the election boards' keys, not the event board's).
Moving it out of the database is out of scope of this feature.

## What a superuser can still do

A change made with superuser or owner rights, to the ballots alone, is caught:

- **The tally refuses it.** Before counting, the tally recomputes the hash of
  every stored ballot of each box and compares it with the manifest whose hash
  is in the signed `BallotBoxSealed` entry on the event's bulletin board. Any
  difference stops that election's tally with a `TallyBallotBoxRejected` ERROR
  entry.
- **Turning the policy off doesn't help.** The tally verifies the boxes of
  every election that has a `BallotBoxSealed` entry on the board, whatever the
  policy says.
- **An old backup is caught.** If the database is restored to before a seal,
  the sealer finds the existing entry on the board and marks the box failed
  instead of sealing it again.

But the tally checks against a bulletin board and a key that it also reads from
the database. A superuser who changes those too can get past it:

1. bypass the triggers, change a sealed box's ballots and replace its seal row
   with a new pending one;
2. point the event's bulletin board reference at another board, and put that
   board's key in the event's place in the secret table;
3. the sealer then signs and posts a new seal for the changed box on the other
   board, and overwrites the seal record (public or restricted). The tally
   reads that board and key, finds one valid seal, and counts the changed box.

The trigger on the bulletin board reference makes this one more trigger to
bypass, not impossible. What still catches it:

- the **original** `BallotBoxSealed` entry, which stays on the original board in
  immudb;
- the **copies** of the seal records and seal hashes that observers kept;
- an auditor running `step-cli step verify-ballot-box-seal` with
  `--expected-key` set to the event key's fingerprint **obtained before** the
  change: the new seal is signed by another key.

Other evidence:

- **The audit trail.** PostgreSQL's audit log records writes (`pgaudit.log =
  'WRITE'`).
- **The seal record no longer matches** the copies observers kept.

Detection is not prevention. Keeping the services on a least-privilege role,
and the superuser and owner logins apart and alerted, is what prevents it.

## Keep the database and the electoral log consistent

A seal is written to Postgres (the seal row) and, about half a second later, to
immudb (the `BallotBoxSealed` entry). If the two are recovered to different
points, the seal breaks, and the product has no way to repair it:

- Postgres loses the seal (an asynchronous replica is promoted, or a
  point-in-time restore) while immudb keeps the entry: the box is re-sealed,
  finds the entry and fails ("A seal for this ballot box is already on the
  bulletin board").
- immudb is restored to before the entry while Postgres keeps the published
  seal: the tally rejects the box.

A common close seals every box at the same moment, so one such event can hit
all of them. So:

- back up and restore Postgres and immudb to consistent points, never one
  without the other;
- during close and seal windows, use synchronous replication, or a failover
  that loses no committed transactions;
- don't plan failovers or restores during close windows;
- before treating a failed seal or a rejected box as tampering, check the
  recovery history (see the
  [runbook](../../../02-election_managers/01-tutorials/22-admin_portal_tutorials_ballot-box-seal-runbook.md#after-a-restore-or-failover)).
