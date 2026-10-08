---
id: admin_portal_tutorials_ballot-box-seal-runbook
title: Ballot Box Seal (runbook)
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

This runbook is for election events with the **Seal at close** policy. It
covers the cases where a ballot box on the election's **Ballot boxes** card
does not reach **Sealed** on its own, or where the tally stays refused. For how
sealing works and what each status means, see
[Ballot box seal](../02-reference/02-election-event/20-ballot-box-seal.md).

| What you see | Section | Is the box locked? | Can it fix itself? |
| --- | --- | --- | --- |
| Card header: "*Channel* is enabled and not closed: stop it to seal the ballot boxes." | [A channel holds the seal](#a-channel-holds-the-seal) | No | No: stop the channel |
| **Sealing overdue**, with a reason | [An overdue seal](#an-overdue-seal) | No new casts | Depends on the reason |
| **Sealed, publishing** for more than a few minutes | [A publication that does not finish](#a-publication-that-does-not-finish) | Yes | Yes, once the cause is fixed |
| **Not sealed: incident**, and the event's incident banner | [A failed seal](#a-failed-seal) | Yes, permanently | No |
| The tally says an area is "not sealed yet", but the card shows no seal for it | [An area added after the close](#an-area-added-after-the-close) | n/a | No |
| Seals fail, or the tally rejects boxes, after a database failover or restore | [After a restore or failover](#after-a-restore-or-failover) | Yes | No |

In every case the tally of that election waits: it needs every ballot box
Sealed.

## How the sealer runs

- **At the close.** The close creates one pending seal per ballot box. Right
  after it commits, a light task (`schedule_ballot_box_seals`, on the Beat
  queue) schedules one `seal_ballot_box` task to run at the box's deadline
  (`eta`), for each box whose deadline is **at most 5 minutes away**. A box
  with a later deadline (a long grace period) is left to the per-minute
  dispatcher: a message waiting for its `eta` is held unacknowledged by a
  worker, and RabbitMQ closes a consumer that holds one for too long.
- **Every minute.** The dispatcher `seal_ballot_boxes` runs every 60 seconds on
  the Beat queue. It lists the pending boxes past their deadline, the sealed
  boxes still to publish, and the failed boxes whose failure entry isn't
  recorded as posted, and queues one `seal_ballot_box` task for each.
- **Each box.** `seal_ballot_box` runs on `short_queue`, with a time limit of
  900 seconds. Every seal message expires 120 seconds after it is due (after
  its `eta`, or after it is sent by the dispatcher, which sends a new one every
  minute). It seals the box if it is due, then publishes it. A
  duplicate is harmless: a run that finds the box held by another run records
  "busy" and stops.
- **What it records.** Each attempt that leaves a box pending records when it
  ran and why the box isn't sealed yet. The card shows that reason (see
  [An overdue seal](#an-overdue-seal)).

### What must run at every close

Before this feature, the services below were needed only at tally time. With
Seal at close they are needed at **every close**:

- the Beat scheduler;
- windmill workers that consume `short_queue`, with free capacity during close
  windows (every worker configuration in this repository consumes it);
- the Keycloak database (the seal counts the eligible voters of each area);
- the key store (the event's protocol manager key, kept in the database's
  secret store under the master secret);
- immudb (the electoral log);
- the public documents bucket (the seal records).

**Upgrade note.** When you enable Seal at close on a deployment, check that its
worker deployment keeps `short_queue` consumers running during every close
window, and does not scale them down outside tallies.

**Worker sizing.** At a common close, every ballot box is due at the same
time, one task per box. Each task holds its box's ballots in memory, and then
its public record. In the measurement below, a 50,000-ballot box was about
33 MB of ballots and produced a 26.6 MB record.

How many seal tasks a worker runs at once depends on its `--prefetch-count`:

- With `--prefetch-count 1` (the development and remote compose files), a
  worker runs **one task at a time across all the queues it consumes**. If it
  also consumes `tally_queue` and is running a tally, no seal runs on it until
  the tally ends. Boxes are sealed one after another per worker. (Messages
  waiting for their `eta` don't count against this limit.)
- Without the option (the production images), a worker takes up to 100
  messages at once, so many boxes can be in memory together.

Make sure that the seal tasks a worker can run at once, times the largest
boxes, fit in its memory, and that enough workers consume `short_queue`, and
are free of long tallies, to seal every box within the time you need. A
dedicated `short_queue` worker during close windows avoids waiting behind a
tally.

**Upgrading to migration `1791000001700`.** The migration replaces triggers on
`cast_vote` and other busy tables, and waits at most 10 seconds for their locks
(`lock_timeout`). If a long transaction holds `cast_vote`, the migration fails;
Hasura's startup migration then fails too, and Hasura restarts in a loop until
the transaction ends. Apply it in a quiet window, not during voting.

## A channel holds the seal

**What you see.** The card's header says "*Channel* is enabled and not closed:
stop it to seal the ballot boxes." The rows stay **Open**, or the Stop Voting
confirmation says "*channel* is still enabled and not closed".

**Why.** A ballot box is sealed only when every enabled channel of its election
is finished. A channel that never started holds the seal, because it could
still open and take votes. The only exception is Early Voting once Online
voting has started: it can no longer open.

**What to do.** Stop the channel that holds the seal on the election's
**Publish** tab, or with a signed Close voting request. With Seal at close you
can stop a channel that never started: it closes without opening ("*channel*
never opened: stopping it means it won't open"). A scheduled close or a signed
request closes such a channel only if it names it, the election enables it,
and another enabled channel of the election ran; it never closes a channel it
doesn't name or the election doesn't enable. A scheduled close leaves an
election that never opened as it is. Once every enabled
channel is finished, the deadline starts and the box is sealed.

## An overdue seal

**What you see.** The deadline has passed and the row says **Sealing
overdue**, with a reason under it. The card's header says that the ballot boxes
are being sealed.

No new ballots are accepted in the meantime: casts are refused once the seal
deadline has passed. The box is not sealed yet, so it is not locked by the
database guard.

| Reason shown | Why | What to do |
| --- | --- | --- |
| *Channel* is still enabled and not closed: stop it to seal the ballot box. | A channel holds the seal. | See [A channel holds the seal](#a-channel-holds-the-seal). |
| *N* votes are in progress in Datafix: the ballot box is sealed once they are resolved. | See [Datafix votes in progress](#datafix-votes-in-progress). | Resolve the votes. |
| The last attempt couldn't …; it is retried every minute. (one text per kind of error, below) | An error the sealer retries. | See [Errors the sealer retries](#errors-the-sealer-retries). The next attempt after the cause is fixed seals the box. |
| Last tried at *time*: the sealer may not be running. Check Beat and the seal worker. | No attempt for more than 3 minutes. | Check that Beat and a worker on `short_queue` are running (see [What must run](#what-must-run-at-every-close)). |
| Not tried yet: the sealer may not be running. Check Beat and the seal worker. | No attempt in the 3 minutes after the deadline. | Same. |

When the row says **Sealing now** ("Being sealed: this takes up to a minute."),
nothing is wrong: the sealer is working on it.

### Errors the sealer retries

When an attempt fails, the card shows only the kind of error; the details are
in the windmill service log, in the warning "The ballot box seal will be
retried: …", which carries the seal ID and the category. The attempt is
retried every minute until it succeeds.

| Category | Card text | What to check |
| --- | --- | --- |
| `board` | The last attempt couldn't reach the bulletin board; it is retried every minute. | immudb is up and reachable from windmill, and the event's board exists; other log entries of the event are delivered. |
| `census` | The last attempt couldn't read the voter list; it is retried every minute. | The Keycloak database is up and reachable, and the event's realm exists. |
| `keystore` | The last attempt couldn't get the signing key; it is retried every minute. | The event's protocol manager key is in the secret store, and windmill has the master secret. |
| `storage` | The last attempt couldn't reach the database or file storage; it is retried every minute. | The public documents bucket (and the database) is reachable from windmill. |
| `settings` | The last attempt couldn't read the election's settings; it is retried every minute. | The event's or election's configuration parses; look for a recent manual or imported change to its presentation. |
| `ballots` | The last attempt found a ballot that can't be read yet or is still in progress; it is retried every minute. | The log names the ballot. A ballot whose content can't be parsed, or that is still in progress, needs the Datafix procedure or an investigation; it is not an incident by itself. |
| `database` | The last attempt couldn't complete in the database; it is retried every minute. | Postgres is up; look for lock waits, timeouts or a failover at that time. |
| `other` | The last attempt failed; it is retried every minute. The service log has the details. | Read the service log. |

### Datafix votes in progress

The area still has cast votes with the status `in-progress`. These are Datafix
votes: they enter as `in-progress` and the Datafix integration later marks them
`valid` or `discarded`. Before sealing, the sealer counts them
(`count_unresolved_cast_votes`, the same check the tally uses). If any remain,
sealing now would fix an outcome that is not known yet, so the seal waits. It
checks again every minute, and logs the warning "Ballot box past its seal
deadline waits for Datafix votes in progress", with the number of votes, when
that number changes.

1. Find the votes in progress for that election and area.
2. Let the Datafix integration finish processing them, or resolve them through
   its documented procedure.
3. Watch the card. Within a minute of the last vote being resolved, the row
   moves to **Sealed, publishing** and then **Sealed**.

## A publication that does not finish

**What you see.** The row stays on **Sealed, publishing** ("The ballot box is
locked. Its entry on the bulletin board is being posted again."), and **Seal
record** says "Not yet".

**What it means.** The ballot box **is sealed**: it is locked and its seal hash
is final and stored. What is still missing is one of the publication steps:

1. posting the signed `BallotBoxSealed` entry to the event's electoral log;
2. reading back the log entry's ID;
3. uploading the public seal record.

Each step is safe to repeat. A retry never creates a second log entry or a
second record: the log delivery and the record's document ID are fixed per
seal. Normally this status lasts less than a minute.

**What to check**, in order:

1. **The workers are running.** The Beat scheduler and a windmill worker on
   `short_queue` must be up. If they were down, the next run after they start
   publishes the box.
2. **The electoral log is reachable.** The entry goes to the event's board in
   immudb. Check that immudb is up and that other log entries (for example the
   voting period close) were delivered.
3. **The public documents storage is reachable.** The record is uploaded to the
   event's public documents bucket, where published results go too.
4. **The windmill logs** for errors of the `seal_ballot_box` task, which carry
   the seal ID. A successful run logs "Ballot box seal published".

Once the cause is fixed, the next run completes the publication. There is
nothing to retry by hand.

## A failed seal

**What you see.**

- The row says **Not sealed: incident**, with a reason, and the card's header
  says "A ballot box could not be sealed: it stays locked, and the incident is
  in the logs."
- The event's **Dashboard** and **Publish** tab show a banner: "*N* ballot
  boxes could not be sealed. This is an incident: each of these ballot boxes
  stays locked and can't be tallied." It lists each box with its reason.
- In a new tally, the election shows **Not sealed: incident** and can't be
  selected.
- The event's **Logs** tab has a `BallotBoxSealFailed` entry at level ERROR:
  "Ballot box of *election*, *area* not sealed, it stays locked: *reason*." It
  is posted once. Windmill also logs "The ballot box could not be sealed: it
  stays locked" at error level. The product sends no other alert: watch for
  the banner, or for ERROR entries in your log monitoring.

**Why it happens.** The reason is one of:

| Reason | Meaning |
| --- | --- |
| A ballot does not match its Ballot ID. | A stored ballot's content no longer produces the Ballot ID stored with it (the same check the cast runs). The stored data was altered or corrupted. The log entry gives both IDs: "stored *X*, content hashes to *Y*". |
| A ballot has no content or no Ballot ID. | A stored ballot is incomplete. |
| The election event has no bulletin board. | There is nowhere to post the seal. |
| A seal for this ballot box is already on the bulletin board. | The event's log already has a `BallotBoxSealed` entry for this box. This happens after the database was restored, or failed over, to a point before the seal: see [After a restore or failover](#after-a-restore-or-failover). The system never posts a second seal. |

A ballot that can't be read, or a configuration that doesn't parse, does not
fail the seal: it is retried (see [An overdue seal](#an-overdue-seal)).

**What the system does.** The ballot box **stays locked**: no ballot can be
added, changed or deleted, so the evidence is preserved. The failure is final
for that box. There is no retry, no reset and no way to unseal it from the
Admin Portal. The election cannot be tallied, because the tally requires every
box to be Sealed.

**What to do.** First check whether the database was restored or failed over
around the time of the seal (see
[After a restore or failover](#after-a-restore-or-failover)). If not, treat it
as a security incident, under the organization's incident procedure.

1. **Don't change the data.** Don't try to edit ballots, the seal row or the
   database to make the seal pass.
2. **Record the evidence.** Note the election, area, reason and time from the
   card, and export the `BallotBoxSealFailed` log entry. Keep a database backup
   and the database audit log (pgaudit) of writes to cast ballots.
3. **Investigate.**
   - For a ballot that does not match its Ballot ID, or is incomplete, compare
     the box's ballots with their `CastVote` log entries (`step-cli step
     export-cast-votes`): each entry holds the ballot's hash and Ballot ID as
     cast. Look in the database audit log for writes to that ballot outside the
     cast path.
   - For an existing seal, find the earlier `BallotBoxSealed` entry in the Logs
     tab.
4. **Decide.** What happens to that election's result is the organization's
   decision, under its incident and legal procedures. The product does not
   offer a way around the failed seal.

Other ballot boxes and elections are not affected.

## An area added after the close

**What you see.** The tally refuses the election with "Not ready: *election*:
*area* (not sealed yet)", but the Ballot boxes card has no seal for that area,
and nothing changes over time.

**Why.** The set of ballot boxes to seal is fixed when the election closes:
the close creates one pending seal per expected box. If a configuration
published **after** the close gives the election a ballot style in a new area,
the tally expects that box too, but no seal is ever created for it, because the
close does not run again. The tally is then refused for good.

The opposite case is refused too: if a sealed box's area loses the election's
contests, the tally says "The sealed ballot box of *election*, *area* is not in
this tally: its area no longer has the election's contests."

**What to do.** Avoid it: don't publish a configuration that changes an
election's areas after it has closed. If it has happened, there is no way in
the Admin Portal to fix it. Escalate it to your platform operator, with the
election, the area and the time of the publication.

## After a restore or failover

Sealing writes to two systems: the database (the seal row, in one transaction)
and immudb (the `BallotBoxSealed` entry, about half a second later). If the two
are restored to different points, the seal breaks:

| What happened | What you see |
| --- | --- |
| The database lost its last seconds (an asynchronous replica was promoted, or a point-in-time restore) but immudb kept the entry | The seal row is pending again. The sealer finds the entry on the board and marks the box failed: "A seal for this ballot box is already on the bulletin board." |
| immudb was restored to a point before the entry, but the database kept the published seal | The tally rejects the box: the bulletin board has no `BallotBoxSealed` entry for it. |

Both look like tampering, but come from operations. Before you treat a failed
seal or a rejected box as an incident, check the database's and immudb's
recovery history around the seal time.

To prevent it:

- restore Postgres and immudb to consistent points, never one without the
  other;
- during close and seal windows, use synchronous replication, or a failover
  that loses no committed transactions;
- avoid planned failovers and restores during close windows.

There is no product path to recover such a box. Escalate it to your platform
operator with the recovery history.

## Performance

Measured once on a development machine (16 cores, a release build, with the
database shared with other work), with real 654-byte single-contest ballots.
These are reference numbers, not a service level:

| Work | Time |
| --- | --- |
| The close, creating the pending seals of 130 ballot boxes | 0.34 s |
| Sealing the largest box (50,000 ballots, 44,500 counted) | 1.83 s, of which 1.45 s re-checks every Ballot ID; the manifest is 7.55 MB |
| Sealing a small box (50 ballots) | 75 ms (median) |
| Sealing all 130 boxes one after another | 11.5 s |
| Publishing the largest box (its record is 26.6 MB of JSON) | 0.42 s |
| Publishing a small box | 0.21 s |
| The database guard on a bulk insert of 10,000 ballots | about +19 µs per row (70.5 against 51.0 µs) |

These were measured before the review fixes of the sealer (the per-box
scheduling at the deadline and the recorded waiting reasons), which don't
change the sealing work itself. Each box is sealed by its own task, so the boxes
of a common close are sealed in parallel across the workers on `short_queue`;
see [worker sizing](#what-must-run-at-every-close).
