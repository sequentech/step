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
| Card header: "*Channel* isn't closed and isn't enabled for this Post: stop it to seal the ballot boxes." | [A channel holds the seal](#a-channel-holds-the-seal) | No | No: stop the channel at the election |
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
  queue) schedules one `seal_ballot_box` task to run at each box's deadline
  (`eta`), however far away it is: the task queues store the `eta`, and the
  waiting task holds no worker.
- **Every minute.** The dispatcher `seal_ballot_boxes` runs every 60 seconds on
  the Beat queue, as a backstop. It lists the pending boxes past their deadline
  and the sealed boxes still to publish, and queues one `seal_ballot_box` task
  for each.
- **Each box.** `seal_ballot_box` runs on `short_queue`, with a time limit of
  900 seconds. Every seal message expires 120 seconds after it is due (after
  its `eta`, or after it is sent by the dispatcher, which sends a new one every
  minute). It seals the box if it is due, then publishes it. A
  duplicate is harmless: a run that finds the box held by another run records
  "busy" and stops, and the event's database holds at most one seal per box.
- **The seal itself.** The seal and its `BallotBoxSealed` entry are written to
  the event's electoral-log database in one transaction, under a lock of the
  box that votes wait for. Before it, the sealer makes sure every ballot the box
  accepted is on the log: it asks the ballot box sequencer for those that
  aren't and waits for them, up to a minute. A failed seal is stored with its
  `BallotBoxSealFailed` entry the same way. The seal row of the Hasura database
  follows; if that update is lost, the next attempt copies the stored seal into
  it.
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
- the event's electoral-log database, which holds its ballot box and its
  electoral log, and the ballot box sequencer, which adds the accepted ballots
  to the log;
- the documents storage: the private documents bucket for restricted seal
  records (the default), the public one for public records.

**Upgrade note.** When you enable Seal at close on a deployment, check that its
worker deployment keeps `short_queue` consumers running during every close
window, and does not scale them down outside tallies.

**Worker sizing.** At a common close, every ballot box is due at the same
time, one task per box. Each task holds its box's ballots in memory, and then
its seal record. In the measurement below, a 50,000-ballot box was about
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
busy tables (`election_event`, `election`), and waits at most 10 seconds for
their locks (`lock_timeout`). If a long transaction holds one of them, the
migration fails; Hasura's startup migration then fails too, and Hasura restarts
in a loop until the transaction ends. Apply it in a quiet window, not during
voting.

**Event databases created before the seal schema** get the seal table and the
guards from `electoral-log-admin upgrade-event-databases`. Run it before the
first close of an event that seals.

## A channel holds the seal

**What you see.** The card's header says "*Channel* is enabled and not closed:
stop it to seal the ballot boxes." The rows stay **Open**, or the Stop Voting
confirmation says "*channel* is still enabled and not closed".

**Why.** A ballot box is sealed only when every channel that counts at its
election is Closed. A channel counts when the election enables it, when it is
Open or Paused, or when it ever started. A channel that never started holds the
seal, because it could still open and take votes: the platform doesn't decide
for you that it is over. This includes Early Voting after Online voting has
started.

A channel the election doesn't enable holds the seal too when it isn't Closed
and it is Open or Paused, or it ran: for example, Kiosk was unchecked in the
election's channels while it was open, after it took ballots. The header then
says "*Channel* isn't closed and isn't enabled for this Post: stop it to seal
the ballot boxes." With Seal at close, an event-wide Start no longer opens a
channel at an election that doesn't enable it, so this comes only from a
change of the election's channels while it ran.

**What to do.** Stop the channel that holds the seal on the election's
**Publish** tab, or with a signed Close voting request. With Seal at close you
can stop a channel that never started: it closes without opening ("*channel*
never opened: stopping it means it won't open"). A scheduled close or a signed
request closes such a channel only if it names it, the election enables it,
and another enabled channel of the election ran; it never closes a channel it
doesn't name or the election doesn't enable. A scheduled close leaves an
election that never opened as it is.

For a channel the election doesn't enable, stop it on the election's
**Publish** tab: with Seal at close, its **Stop Voting** menu offers every
channel that isn't Closed, enabled or not. Leave the election's enabled
channels as they are: closing lets no ballot in, while enabling the channel
would open it to voters until you stop it. A scheduled close or a signed Close voting
request doesn't close it, because they close only the channels the election
enables.

Once every channel that counts is Closed, the deadline starts and the box is
sealed.

## An overdue seal

**What you see.** The deadline has passed and the row says **Sealing
overdue**, with a reason under it. The card's header says that the ballot boxes
are being sealed.

No new ballots are accepted in the meantime: casts are refused once the seal
deadline has passed. The box is not sealed yet, so it is not locked by the
guard of the event's database.

| Reason shown | Why | What to do |
| --- | --- | --- |
| *Channel* is still enabled and not closed: stop it to seal the ballot box. | A channel holds the seal. | See [A channel holds the seal](#a-channel-holds-the-seal). |
| *Channel* isn't closed and isn't enabled for this Post: stop it to seal the ballot box. | A channel the election doesn't enable is open or paused, or ran, and isn't closed. | Stop it at the election, as it is: see [A channel holds the seal](#a-channel-holds-the-seal). |
| *Channel* has ballots in this ballot box and isn't closed: stop it to seal the ballot box. | The box holds ballots cast on that channel, and the channel isn't Closed. The sealer checks every ballot's channel before it seals. | Stop that channel at the election; the Stop Voting menu offers it whether or not the election enables it. A ballot stored without a channel (from before ballots recorded their channel) counts as Online, so the reason may name Online on an election that doesn't enable it: stop Online there. If the channel shows Closed, its status was changed outside the Admin Portal: treat it as an incident. |
| *N* votes are in progress in Datafix: the ballot box is sealed once they are resolved. | See [Datafix votes in progress](#datafix-votes-in-progress). | Resolve the votes. |
| *N* ballots aren't on the bulletin board yet: the ballot box is sealed once the sequencer has added them. | See [Ballots not on the log yet](#ballots-not-on-the-log-yet). | Check the sequencer. |
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
| `board` | The last attempt couldn't reach the bulletin board; it is retried every minute. | The event's electoral-log database is up and reachable from windmill (`ELECTORAL_LOG_PG_*`), and the event has its database and board (`electoral-log-admin`); other log entries of the event are delivered. |
| `census` | The last attempt couldn't read the voter list; it is retried every minute. | The Keycloak database is up and reachable, and the event's realm exists. |
| `keystore` | The last attempt couldn't get the signing key; it is retried every minute. | The event's protocol manager key is in the secret store, and windmill has the master secret. |
| `storage` | The last attempt couldn't upload the seal record to the file storage; it is retried every minute. | The documents buckets are reachable from windmill. |
| `settings` | The last attempt couldn't read the election's settings; it is retried every minute. | The event's or election's configuration parses; look for a recent manual or imported change to its presentation. |
| `ballots` | The last attempt found a ballot that can't be read yet or is still in progress; it is retried every minute. | The log names the ballot. A ballot whose content can't be parsed, or that is still in progress, needs the Datafix procedure or an investigation; it is not an incident by itself. |
| `database` | The last attempt couldn't complete in the database; it is retried every minute. | The Hasura database is up; look for lock waits, timeouts or a failover at that time. |
| `other` | The last attempt failed; it is retried every minute. The service log has the details. | Read the service log. |

### Datafix votes in progress

The area still has ballots with the status `pending` in the event's ballot
box. These are Datafix votes: they enter as `pending` and the Datafix
integration later marks them `valid` or `rejected`. Before sealing, the sealer
counts them (the same check the tally uses). If any remain,
sealing now would fix an outcome that is not known yet, so the seal waits. It
checks again every minute, and logs the warning "Ballot box past its seal
deadline waits for Datafix votes in progress", with the number of votes, when
that number changes.

1. Find the votes in progress for that election and area.
2. Let the Datafix integration finish processing them, or resolve them through
   its documented procedure.
3. Watch the card. Within a minute of the last vote being resolved, the row
   moves to **Sealed, publishing** and then **Sealed**.

### Ballots not on the log yet

A seal comes after every ballot it seals on the event's electoral log. The
ballot box accepts a vote first, and the sequencer then signs and appends its
`CastVote` entry, normally within seconds. If ballots of the box still wait for
the sequencer, the sealer asks it to run and waits up to a minute; if they are
still not on the log, the box stays pending with this reason, and the next
attempt tries again.

1. Check that the ballot box sequencer runs: the Beat task
   `schedule_ballot_box_sequencers`, and a windmill worker that consumes
   `electoral_log_batch_queue`, where its `sequence_ballot_box` tasks run.
2. Check that the event's electoral-log database is up and reachable from
   windmill.
3. Watch the card. Once the sequencer has added the ballots, the next attempt
   seals the box.

## A publication that does not finish

**What you see.** The row stays on **Sealed, publishing** ("The ballot box is
locked and its seal is on the bulletin board. Its seal record is being
published."), and **Seal record** says "Not yet".

**What it means.** The ballot box **is sealed**: it is locked, its seal hash is
final and stored, and its signed `BallotBoxSealed` entry is on the event's
electoral log, stored with the seal. What is still missing is one of the
publication steps:

1. reading the seal's entry back from the event's electoral-log database, and
   checking that it is the stored seal's;
2. uploading the seal record, as a private event document (Seal Record
   Publication policy **Restricted**, the default) or to the public documents
   bucket (**Public**).

Each step is safe to repeat. A retry never creates a second record: the
record's document ID is fixed per seal. Normally this status lasts less than a
minute.

**What to check**, in order:

1. **The workers are running.** The Beat scheduler and a windmill worker on
   `short_queue` must be up. If they were down, the next run after they start
   publishes the box.
2. **The electoral log is reachable.** The entry is in the event's
   electoral-log database. Check that it is up and reachable from windmill.
   If the windmill log says the seal record is not on the bulletin board, or is
   not the seal's, treat it as an incident: see
   [After a restore or failover](#after-a-restore-or-failover).
3. **The documents storage is reachable.** A restricted record is uploaded to
   the private documents bucket, a public one to the event's public documents
   bucket, where published results go too. The event's
   [Seal Record Publication policy](../02-reference/02-election-event/20-ballot-box-seal.md#the-seal-record-publication-policy)
   says which.
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
  is stored once, with the failed seal. Windmill also logs "The ballot box
  could not be sealed: it stays locked" at error level. The product sends no other alert: watch for
  the banner, or for ERROR entries in your log monitoring.

**Why it happens.** The reason is one of:

| Reason | Meaning |
| --- | --- |
| A ballot does not match its Ballot ID. | A stored ballot's content no longer produces the Ballot ID stored with it (the same check the cast runs). The stored data was altered or corrupted. The log entry gives both IDs: "stored *X*, content hashes to *Y*". |
| A ballot has no content or no Ballot ID. | A stored ballot is incomplete. |
| A ballot has an unknown voting channel. | A stored ballot names a channel that is none of Online, Kiosk, Early Voting or Telephone. The stored data was altered or corrupted. The log entry names the channel. |
| A ballot has no cast-vote record on the bulletin board. | The sequencer is done with a ballot of the box, but its `CastVote` entry is not on the event's board. The log lost an entry or was altered. The log entry gives the number of such ballots. |
| The election event has no bulletin board. | There is nowhere to post the seal. Without a board the event takes no ballots, and this failure is recorded on the seal row only. |
| A seal for this ballot box is already on the bulletin board. | The event's board already has the box's `BallotBoxSealed` entry, but its database has no seal for the box. The seal and its entry are written in one transaction, so the seal was removed past the database's guards. The system never posts a second seal. |

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
   card, and export the `BallotBoxSealFailed` log entry. Keep a backup of the
   event's electoral-log database and the database audit log (pgaudit) of
   writes to its ballots.
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

A seal and its `BallotBoxSealed` entry are written to the event's electoral-log
database in one transaction, so restoring that database never separates them.
The seal row of the Hasura database follows in a second step. If the two
databases are restored to different points:

| What happened | What you see |
| --- | --- |
| The Hasura database lost its last seconds (an asynchronous replica was promoted, or a point-in-time restore) but the event's database kept the seal | The seal row is pending again. The sealer's next attempt finds the stored seal and copies it into the row: **Sealed, publishing**, then **Sealed**. Nothing to do. |
| The event's electoral-log database was restored to a point before the seal, but the Hasura database kept it | The box has no seal in its database, so its guard no longer refuses writes. Casts stay refused (the cast check reads the seal deadline from the Hasura database). The tally rejects the box ("the ballot box's seal row says it is sealed, but the stored seal is missing"), and the electoral-log audit reports that the board no longer matches its published checkpoints. |

The second case looks like tampering, but can come from operations. Before you
treat a rejected box as an incident, check the recovery history of both
databases around the seal time. Then restore the event's database to a point
after the seal, or escalate it to your platform operator.

To prevent it:

- restore the Hasura database and the event databases to consistent points,
  never one without the other;
- during close and seal windows, use synchronous replication, or a failover
  that loses no committed transactions;
- avoid planned failovers and restores during close windows.

## Performance

Measured once with the `perf_ballot_box_seal` test on a development machine,
in a **debug** build, with synthetic contest-less ballots, and the Hasura and
electoral-log databases in one PostgreSQL 18 container. A release build is
faster. These are reference numbers, not a service level:

| Work | Time |
| --- | --- |
| 50,000 votes into one box, one `accept_ballot` statement each, one after another | 35.7 s |
| The sequencer signing and appending 56,450 `CastVote` entries | 23.2 s |
| The close, creating the pending seals of 130 ballot boxes | 0.05 s |
| Sealing the largest box (50,000 ballots, 44,500 counted) | 2.65 s, of which 1.3 s reads and re-checks every Ballot ID and hash; the manifest is 7.55 MB |
| Sealing a small box (50 ballots) | 16 ms (median) |
| Sealing all 130 boxes one after another | 4.8 s |
| Publishing the largest box (its record is 26.6 MB of JSON) | 1.55 s |
| Publishing a small box | 12 ms |

The guard costs a shared advisory lock and a seal lookup per ballot write. In
that test, 1,000 votes took 627 to 793 µs each with or without the guard,
within the noise; a load test of 2 KB single-row inserts with 8 clients
measured 8,000 to 8,300 writes per second with the guard against 8,200 without.

Each box is sealed by its own task, so the boxes of a common close are sealed
in parallel across the workers on `short_queue`; see
[worker sizing](#what-must-run-at-every-close).
