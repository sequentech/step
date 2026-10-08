---
id: ballot_box_seal
title: Ballot box seal
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

When voting closes, Step can seal each ballot box. A **ballot box** is the cast
ballots of one election in one area. Sealing does three things:

1. It locks the ballot box in the database. From then on no ballot can be added,
   changed or deleted, by any application path.
2. It computes one SHA-512 hash, the **seal hash**, over a manifest that lists
   every ballot in the box and says how each one counts.
3. It posts a signed, time-stamped `BallotBoxSealed` entry with that hash to the
   election event's electoral log (the bulletin board), and publishes a **seal
   record** that can be checked offline. By default the record is restricted
   to administrators, who share it with observers; the
   [Seal Record Publication policy](#the-seal-record-publication-policy) can
   make it public.

The tally then counts only sealed ballot boxes, and first checks each one
against its signed entry.

:::note
These are the electronic ballot boxes that hold cast votes. They are not the
digitalized paper or postal **Ballot Boxes** of the
[Tally Sheets tab](../../01-tutorials/21-admin_portal_tutorials_ballot-boxes.md).
:::

Related pages:

- [Verify a ballot box seal](../../../04-election_auditors/02-tutorials/06-auditor_verify-ballot-box-seal.md):
  check a seal record without the system.
- [Ballot box seal runbook](../../01-tutorials/22-admin_portal_tutorials_ballot-box-seal-runbook.md):
  an overdue seal, a stuck publication, a failed seal, an area added after the
  close, a database restore.
- [Demonstrate the ballot box seal](../../01-tutorials/23-admin_portal_tutorials_ballot-box-seal-demonstration.md):
  close, seal, refuse writes, verify offline and tally, step by step.
- [Database role for the ballot box seal](../../../05-reference/05-cryptography_security/02-security/03-ballot-box-seal-database-role.md):
  the deployment prerequisites, the upgrade note and what a database superuser
  can still do.

## The Ballot Box Seal Policy

Set it in **Election Event > Data > Advanced Configurations**, after **Lockdown
Status**.

| Value | Effect |
| --- | --- |
| **Do not seal** (default) | Nothing changes. No seal is created, the tally works as before, and the Admin Portal shows no sealing texts. Existing and imported events without the setting use this value. |
| **Seal at close** | When voting closes, the ballot box of each area is sealed, Closed becomes final for every election of the event, and the tally requires published seals. |

The policy applies to the whole election event. An event created from a
template or preset that sets **Seal at close** starts with it.

**The policy is locked once voting has opened.** You can change it only while
voting has never started on the event or on any of its elections, on any
channel. After that the select is disabled, and the reason is shown under it:

- "Locked: voting has opened in *names*." (the elections where it opened);
- "Locked: voting has opened in this election event.";
- "Locked: the elections could not be read, so whether voting has opened is
  unknown." While they load: "Checking whether voting has opened…".

A change through the API or the database is refused by the database too, with
"The ballot box seal policy of election event *id* can only change before
voting opens" (SQLSTATE `42501`). The Admin Portal shows that refusal as "The
Ballot Box Seal Policy can't be changed after voting has opened." There is no
exception, not even for the server.

**Other settings locked with Seal at close.** Once voting has opened on a Seal
at close event, these can't change either, because the seal and the tally rely
on them:

- the **contest encryption policy**, the **delegated voting policy** and the
  **weighted voting policy**. In **Data > Advanced Configurations** their
  selects are disabled, with the reason "With Seal at close, the seal relies on
  this setting." A refused change says "This setting can't be changed after
  voting has opened: with Seal at close, the seal relies on it."
- the event's **bulletin board**: "The election event's bulletin board can't
  change after voting has opened: with Seal at close, the seals are posted to
  it."
- the [Seal Record Publication policy](#the-seal-record-publication-policy).

The database compares the **effective** values: a setting that is missing or
empty counts as its default. So saving the event for any other reason (a logo,
a text, the lockdown or enrollment schedule, the results website), which may
write a setting out with its default value, is not refused.

There is no way to turn sealing off for an event once voting has opened, and no
way to unseal a ballot box. Choose the policy before you open voting.

## The Seal Record Publication policy

The **Ballot Box Seal Record** select, under the Ballot Box Seal Policy in
**Election Event > Data > Advanced Configurations**, decides who can download
the seal record of each sealed ballot box. It is shown only when the Ballot Box
Seal Policy is **Seal at close**. It is stored as
`presentation.ballot_box_seal_record_policy`.

| Value | Who can download the record | Where it is |
| --- | --- | --- |
| **Restricted** (default) | "Only administrators can download it; share it with observers." Users with the document download permission download it from the Ballot boxes card, through a short-lived signed link. | A private document of the election event, in the private documents bucket. The seal has no public path. |
| **Public** | "Anyone with the event's ids can download it, without signing in; it shows how each ballot counted." | The event's public documents bucket, at `tenant-<tenant id>/event-<event id>/ballot-box-seals/<election id>/<area id>.json`. |

An event without the setting, including existing and imported events, uses
**Restricted**. Saving the event without touching the select doesn't add the
setting.

The record is the same file under both values: the seal hash, the signed log
entry and, for each ballot, its hash, Ballot ID, how it counts (counted,
replaced, not eligible, discarded), its weight and its channel. Only who can
read it differs. The policy changes nothing in the seal, the electoral log or
the tally. See
[what the record reveals](../../../04-election_auditors/02-tutorials/06-auditor_verify-ballot-box-seal.md#privacy-what-the-record-reveals)
before choosing **Public**.

**It is locked once voting has opened**, like the Ballot Box Seal Policy and
with the same note under the select ("Locked: voting has opened in …"), so
every ballot box of the event is published the same way. The database compares
its effective value (missing counts as **Restricted**) and refuses a change
with "The ballot_box_seal_record_policy of election event *id* can't change
after voting has opened"; the Admin Portal shows "This setting can't be changed
after voting has opened: with Seal at close, the seal relies on it."

**Upgrading.** Before this setting existed every record was public. Records
already published keep their public path. A box sealed but not yet published
when the upgrade is applied is published under the event's current value,
which is **Restricted** unless the event sets **Public**.

## When a ballot box is sealed

A ballot box is sealed once **both** conditions hold for its election:

- voting has **finished**: every channel that counts is **Closed**, and at
  least one channel counts;
- the **seal deadline** has passed.

A channel **counts** when any of these is true:

- the election **enables** it;
- it is **Open** or **Paused**;
- it has **ever started**, even if the election no longer enables it (it may
  have taken ballots).

A channel that counts and isn't Closed **holds the seal** until it is closed.
Nothing is guessed on your behalf: an enabled channel that never started holds
the seal too, Early Voting included, also once Online voting has started.

A channel that was unchecked in the election's enabled channels while it was
open, or after it took ballots, still holds the seal. Enable it again in the
election's settings and stop it: the Stop dialog and the Ballot boxes card say
"*Channel* is open but not enabled for this Post: enable it again and stop it
to seal the ballot boxes."

The sealer also checks the ballots themselves. If the ballot box holds a ballot
cast on a channel that isn't Closed, the box stays pending and the card says
"*Channel* has ballots in this ballot box and isn't closed: stop it to seal the
ballot box." A ballot whose channel is none of Online, Kiosk, Early Voting or
Telephone is an incident: the seal fails, as for a Ballot ID mismatch.

With Seal at close, you can close a channel that never started directly, so
it doesn't hold the seal:

- **Stop Voting** is offered for it. The confirmation says "*channels* never
  opened: stopping it means it won't open."
- A **scheduled close** or a **signed Close voting request** closes it only if
  the schedule or the request **names that channel**, the election **enables**
  it, and another **enabled** channel of the same election **actually ran**.
  - A close never touches channels it doesn't name: the scheduled end of Early
    Voting doesn't close Online.
  - A close never touches channels the election doesn't enable: a signed close
    with the default channels (Online and Kiosk) on an election that enables
    only Online leaves Kiosk alone.
  - An election that never opened is left as it is by its scheduled close. The
    log says "Nothing to close at *election* on schedule: it never opened, so
    it stays as it is".

On events with Do not seal this is unchanged: a channel that never started
can't be stopped.

Once an election has seals, none of its channels can be opened, including one
that never started.

The seal deadline is the close time plus the grace period:

- The **close time** is the latest close of the channels that count and ran. A
  channel closed without ever starting took no votes, so it doesn't move the
  close time.
- The **grace period** is the election's grace period. It applies only when the
  Online channel **actually ran**, and the grace period policy is not *No grace
  period*. It applies also when the election no longer enables Online, since
  online ballots were cast under it. An Online channel closed without ever
  opening took no online votes, so it adds no grace period. Otherwise the
  deadline is the close time.

With Seal at close, a cast is refused once the box's seal deadline has passed,
even if the election's end date and grace period would allow it later. So no
ballot is accepted after the deadline that the seal records.

Every way an election closes leads to the seal:

- **Stop Voting** on the election's **Publish** tab;
- **Stop Voting** on the election event's **Publish** tab (all its elections);
- a signed **Close voting** request, when closing signatures are required (see
  [Signatures](./16-signatures/01-election_management_election-event_signatures.md));
- the **scheduled close** (see
  [Scheduled openings and closings with signatures](./18-signed-scheduled-transitions.md)).

**How fast.** The close creates one pending seal per ballot box in the same
transaction. Right after it commits, when a box's deadline is at most 5
minutes away, a seal task is scheduled to run at that deadline, so the box is
sealed within moments of it. A background job also checks every minute for
boxes that are due: it seals boxes with a later deadline (a long grace period)
within about a minute of it, and makes sure a lost task or a worker restart
does not leave a box unsealed. Each box is sealed by its own task. See the runbook for
[how the sealer runs](../../01-tutorials/22-admin_portal_tutorials_ballot-box-seal-runbook.md#how-the-sealer-runs).

**Which ballot boxes are sealed.** One per area that has a published ballot
style with votable contests for the election, plus every area that already
holds cast ballots for it. Empty ballot boxes are sealed too, so the record
proves that they were empty. Each ballot box is sealed exactly once. The set is
fixed at the close: see the runbook about
[an area added after the close](../../01-tutorials/22-admin_portal_tutorials_ballot-box-seal-runbook.md#an-area-added-after-the-close).

## Stop and Start Voting with Seal at close

With **Seal at close**, the Stop Voting confirmation says what this stop does
to the seal. It promises the seal only when the stop finishes voting:

| Case | Text |
| --- | --- |
| One election, no grace period | You are about to stop voting in *name*. Its ballot boxes are then sealed: no ballot can be added, changed or deleted, and voting cannot start again. |
| One election, with a grace period (only when Online ran: see the grace period rule above) | You are about to stop voting in *name*. Its ballot boxes are sealed when the grace period ends, *N* minutes later: from then on no ballot can be added, changed or deleted. Voting cannot start again. |
| One election, another channel still holds the seal | You are about to stop voting period. With Seal at close, its ballot boxes are sealed once every enabled channel is closed: *channels* is still enabled and not closed. |
| One election, a channel it doesn't enable holds the seal | You are about to stop voting period. *Channel* is open but not enabled for this Post: enable it again and stop it to seal the ballot boxes. |
| The stop closes a channel that never opened | The text above is preceded by "*channels* never opened: stopping it means it won't open." |
| Whole event, no grace period | You are about to stop voting in every election. Their ballot boxes are then sealed: no ballot can be added, changed or deleted, and voting cannot start again. |
| Whole event, with grace periods | You are about to stop voting in every election. Their ballot boxes are sealed when each election's grace period ends, up to *N* minutes later: … |
| Whole event, some elections never opened | The text also says "*names* never opened: stopping closes them and seals their empty ballot boxes." The event-wide Stop closes those elections too, and they can't open afterwards. |
| Whole event, some elections keep a channel open | You are about to stop voting in every election. The ballot boxes of *names* are then sealed (or: sealed when their grace period ends, up to *N* minutes later). *Names* keep another channel enabled and not closed: their ballot boxes are sealed once those channels are closed. |
| Whole event, an election keeps open a channel it doesn't enable | The text also says, per such channel: "In *election*, *channel* is open but not enabled: enable it again for that Post and stop it to seal its ballot boxes." |

When closing signatures are required, Close voting still goes through its
signing panel; the panel itself does not change.

**Closed is final.** With Seal at close, a Closed channel stays Closed. The
database refuses any change of a Closed channel's status
(`ballot_box_seal_closed_is_final`), and the server refuses to start voting
again on a Closed election or event, also before the seal, during the grace
period: "Voting can't start again: with the Ballot Box Seal Policy set to Seal
at close, voting that has closed stays closed and its ballot boxes are sealed."
For an election whose ballot boxes are sealed or being sealed, the Admin Portal
doesn't offer **Start Voting**. A close by mistake cannot be undone. Use
**Pause Voting** for a temporary stop: a paused election is not closed and is
not sealed.

**Starting or resuming the whole event** opens only the elections that are not
closed:

- The Start confirmation warns, per election, what stays closed: "With Seal
  at close, closed voting stays closed: *election*: *channels* stays closed;
  *election* stays closed, as its ballot boxes are sealed." It names each
  election where a channel being started is Closed (and that channel), and
  each election that has seals. The other channels of an election still
  open.
- Afterwards the Publish tab shows an alert, **Some elections stay closed**,
  with a line per election: "*name* stays closed: its voting has closed and
  Seal at close makes closing final."
- A scheduled opening records the elections it kept closed in its outcome and
  log.

## The Ballot boxes card

With **Seal at close**, the election's **Dashboard** shows a **Ballot boxes**
card under the charts, in both monitoring modes. It lists the election's ballot
boxes, one row per area, from before the close on. With **Do not seal** the
card is not shown.

The header says where the election is:

| When | Text |
| --- | --- |
| Voting hasn't opened | Voting hasn't opened yet. The ballot box of each area is sealed when voting closes. |
| Voting is open | Voting is open on *channels*. The ballot box of each area is sealed when voting closes. |
| Voting is paused | Voting is paused. The ballot box of each area is sealed when voting closes. |
| A channel holds the seal | *Channels* is enabled and not closed: stop it to seal the ballot boxes. |
| A channel the election doesn't enable holds the seal | *Channel* is open but not enabled for this Post: enable it again and stop it to seal the ballot boxes. (Added to the text above when an enabled channel holds the seal too.) |
| During the grace period | Voting closed at *time*. The ballot boxes are sealed when the grace period ends, at *deadline*. |
| Past the deadline, no grace period | Voting closed at *time*. The ballot boxes are being sealed. |
| Past the deadline, after a grace period | Voting closed at *time*. The grace period ended at *deadline*; the ballot boxes are being sealed. |
| All sealed | Voting closed at *time*. The ballot boxes are sealed: no ballot can be added, changed or deleted. |
| A seal failed | Voting closed at *time*. A ballot box could not be sealed: it stays locked, and the incident is in the logs. |

A second line names who closed the election:

- "Closed by *names* with their certificates, signing code *code*." for a
  signed Close voting request;
- "Closed by *username*." for Stop Voting without signatures;
- "Closed by the scheduled close of polls."

Times are shown in the election's timezone, as on the other election screens.

### Row statuses

Each row shows its status and, under it, why the box is in that state.

| Status | Meaning | Box locked? | Shown under it |
| --- | --- | --- | --- |
| **Open** | Voting has not finished. | No | |
| **Sealing at** *time* | Closed; waiting for the grace period to end. Eligible late casts are still accepted until then. | No | |
| **Sealing now** | Past the deadline; the sealer is sealing it. | No new casts | "Being sealed: this takes up to a minute." |
| **Sealing overdue** | Past the deadline, and something holds the seal. | No new casts | The reason: see below. |
| **Sealed, publishing** | Sealed and locked. The entry on the bulletin board and the seal record are being posted, and are retried until they exist. | Yes | "The ballot box is locked. Its entry on the bulletin board is being posted again." |
| **Sealed** | Sealed, on the bulletin board, and the seal record is published (restricted or public, by the Seal Record Publication policy). | Yes | |
| **Not sealed: incident** | The seal was stopped. | Yes, and it stays locked | The reason, for example "A ballot does not match its Ballot ID." |

The sealer records on each pending seal when it last tried and why the box
isn't sealed yet. The card shows **Sealing overdue** with that reason:

| Recorded reason | Shown under the status |
| --- | --- |
| A channel is still enabled and not closed | *Channel* is still enabled and not closed: stop it to seal the ballot box. |
| A channel the election doesn't enable is open or paused, or ran, and isn't closed | *Channel* is open but not enabled for this Post: enable it again and stop it to seal the ballot box. |
| The ballot box has ballots of a channel that isn't closed | *Channel* has ballots in this ballot box and isn't closed: stop it to seal the ballot box. |
| Datafix votes are in progress | *N* votes are in progress in Datafix: the ballot box is sealed once they are resolved. |
| The last attempt failed and will be retried | A text for the kind of error, for example "The last attempt couldn't reach the bulletin board; it is retried every minute." The kinds are: the bulletin board, the voter list, the signing key, the database or file storage, the election's settings, a ballot that can't be read yet, the database, and other errors. The details are only in the service log. |
| The last attempt is more than 3 minutes old | Last tried at *time*: the sealer may not be running. Check Beat and the seal worker. |
| No attempt yet, more than 3 minutes after the deadline | Not tried yet: the sealer may not be running. Check Beat and the seal worker. |

The runbook explains each case:
[an overdue seal](../../01-tutorials/22-admin_portal_tutorials_ballot-box-seal-runbook.md#an-overdue-seal).

### Columns

| Column | Content |
| --- | --- |
| Area | The area name. |
| Status | One of the statuses above, with the reason under it. |
| In the box | Every ballot in the box, in any status. |
| Counted | The ballots that count: each eligible voter's latest valid ballot. The others were replaced by the voter's later ballot, discarded, or cast by a voter who is not eligible. |
| Sealed | The seal time. |
| Seal hash | The first and last characters of the seal hash, with a copy button for the full value. |
| Seal record | "Not yet" until it is published. Then, for a public record, a link to it; for a restricted record, a button that downloads it through a short-lived signed link, or "Restricted: ask an administrator who can download documents." for users without the document download permission. A failed download says "The seal record could not be downloaded. Try again." |

The seal record is a JSON file. A public one is in the event's public documents
bucket, at
`tenant-<tenant id>/event-<event id>/ballot-box-seals/<election id>/<area id>.json`.
A restricted one is a private document of the event, with the same name. Its
document id is fixed per seal (the seal's `public_document_id`, whatever the
policy), so a retry never adds a second document and the path does not change.

The card is visible to users who can see the election's dashboard or read
tallies.

### The incident banner

When any ballot box of the event could not be sealed, the event's
**Dashboard** and **Publish** tab show a banner: "*N* ballot boxes could not be
sealed. This is an incident: each of these ballot boxes stays locked and can't
be tallied. Follow the runbook for a failed seal." It lists each box as
"*election*, *area*: *reason*". See
[a failed seal](../../01-tutorials/22-admin_portal_tutorials_ballot-box-seal-runbook.md#a-failed-seal).

## What is refused after the seal

The lock is a database guard on the cast ballots of each sealed ballot box. It
applies to every application path, including Hasura roles such as admin-user.

| Action | Result |
| --- | --- |
| A voter casts a ballot into a sealed box, or after its seal deadline | The voter gets the usual "voting is closed" error. `CastVoteError` entries are logged (the cast route retries, so there can be more than one). |
| A Datafix status change on a ballot in a sealed box | Refused: "The ballot box is sealed: voting is closed and no ballot can be added, changed or deleted". |
| Editing, disabling or releasing a voter who has ballots in sealed boxes | The voter record is saved. The voter's ballots in sealed boxes are kept as sealed; ballots in boxes that are not sealed are discarded as usual. If a ballot is kept, the voter stays marked as voted. A warning in the server log names the sealed boxes. |
| Deleting an election that has seals (in any status) | The Admin Portal checks for seals before it offers the delete, and says "This election has sealed ballot boxes and cannot be deleted. Archive its election event instead." The database refuses the delete through any other path too. |
| Deleting the election event when it has seals (in any status) | The Admin Portal checks for seals before it offers the delete: "This election event has sealed ballot boxes and cannot be deleted. Archive it instead." If the seals can't be checked, it warns that the delete is refused if there are any. The server refuses it before anything is deleted. Archiving still works. |
| Any direct insert, update, delete or truncate of cast ballots in a sealed box | Refused by the database (`ballot_box_sealed`, SQLSTATE `42501`). |
| Starting voting again | Refused, also before the seal (see *Closed is final*). |

Ballot boxes of other areas, and of elections that are still open, keep working.

**Eligibility is fixed at the seal.** The seal records which ballots count and
with what weight (for delegated or weighted voting). Disabling a voter or moving
them to another area after the seal does not change the count. Challenges after
the close need the organization's own procedure.

## The tally gate

With Seal at close, an election can be tallied only when **every** ballot box
of the election is **Sealed** (published on the bulletin board), and every seal
of the selected elections is published. The server enforces this when the tally
is created and again when it starts. Its refusal lists what is not ready, for
example "Every ballot box must be sealed and its seal on the bulletin board
before tallying. Not ready: *election*: *area* (not sealed yet)."

The tally is also refused when a sealed ballot box would be left out of it,
because its area no longer has the election's contests: "The sealed ballot box
of *election*, *area* is not in this tally: its area no longer has the
election's contests." A sealed box is never dropped silently.

In **Election Event > Tally**, a new tally lists the elections with a **Ballot
boxes** column:

| Text | Meaning | Selectable |
| --- | --- | --- |
| *sealed* of *total* sealed | All published. | Yes |
| Sealed, *published* of *total* on the bulletin board | All sealed, some still publishing. | No |
| Sealing at *time* | Waiting for the grace period. | No |
| Sealing overdue | Past the deadline and not sealed yet. | No |
| Not sealed: incident | A ballot box could not be sealed. | No |
| Not sealed | Voting hasn't closed. | No |
| Seals unavailable | The seals could not be read. Reload the page. | No |

An election that is not ready has its checkbox disabled and unchecked. A line
under the list names each one and why, for example "*name*: its ballot boxes
are past their deadline and not sealed yet (see its Dashboard)."

Initialization reports are not gated and don't verify seals: they run before
voting.

**What the tally checks.** For the election results tally, for each ballot box,
the tally:

1. reads the box's `BallotBoxSealed` entries from the event's log. Identical
   copies count once, and entries not signed by the event's key are ignored
   (with a warning). There must be exactly one distinct, validly signed entry;
2. checks its signature, and that the sender is the event's system key;
3. checks that the stored manifest has the signed hash and counts, and follows
   the manifest rules (times in order, no more counted ballots than eligible
   voters, a weight of at least 1 on every counted ballot);
4. recomputes the hash of every stored ballot and checks that the ballots are
   exactly the manifest's entries;
5. counts the manifest's counted entries, with their weights.

These checks run when the policy is Seal at close, **or** when the event's log
already holds a `BallotBoxSealed` entry for the election.

If the ballots differ from the seal, that election's tally stops with "The
ballot box of *election*, *area* does not match its seal", and a
`TallyBallotBoxRejected` ERROR entry says what differs. If they match, a
`TallyBallotBoxVerified` entry is logged once per ballot box, and the seal hash
is kept in the tally results' annotations for reports. An operational problem
(no seal, a seal not yet published, the log unreachable) also stops the tally
with an error, but logs no `TallyBallotBoxRejected` entry: nothing was found to
differ.

What the tally checks against (the bulletin board and the key) comes from the
database. See
[what a database superuser can still do](../../../05-reference/05-cryptography_security/02-security/03-ballot-box-seal-database-role.md#what-a-superuser-can-still-do).

## Electoral log entries

The **Logs** tab shows four new statement kinds, filterable like the others.
Click **Show More** in an entry's Description to see the full description and a
plain-words explanation: the seal hash and counts (and, when the box holds
more ballots than it counts, why the others don't count), the Close voting
request, the failure reason, the tally session.

| Kind | Level | Description |
| --- | --- | --- |
| `BallotBoxSealed` | INFO | Ballot box of *election*, *area* sealed: *counted* of *in the box* ballots counted. |
| `BallotBoxSealFailed` | ERROR | Ballot box of *election*, *area* not sealed, it stays locked: *reason*. |
| `TallyBallotBoxVerified` | INFO | Ballot box of *election*, *area* matches its seal: *counted* ballots counted. |
| `TallyBallotBoxRejected` | ERROR | The ballot box of *election*, *area* does not match its seal: *what differs*. |

All four are system entries, signed by the election event's protocol manager
key, like every electoral log entry. "Counted" is the number of counted
ballots, not their total weight. A failed seal's entry is posted once; if the
post is lost, the sealer posts it again until it is recorded.

## Every outcome, explained

| Situation | What happens | Why | What you see |
| --- | --- | --- | --- |
| Policy is Do not seal | No seal; tally unchanged. | Sealing is opt-in per event. | No Ballot boxes card; no sealing texts in Stop Voting; no readiness rule in the tally list. |
| You change the policy before voting has ever opened | Saved. | Nothing has been cast yet. | The select is enabled. |
| You change the policy after voting has opened | Refused. | Changing the rule mid-election would make some boxes sealed and others not, or remove a promised seal. | The select is disabled, with the reason ("Locked: voting has opened in …"). |
| Seal Record Publication policy unset or **Restricted** | Each record is a private event document; the seal has no public path. | Restricted is the default: the record shows how each Ballot ID counted. | Seal record: a download button for users with the document download permission, else "Restricted: …". |
| Seal Record Publication policy **Public** | Each record is uploaded to the public bucket. | The organizers chose to publish it. | Seal record: a link to the public file. |
| You change the Seal Record Publication policy after voting has opened (Seal at close) | Refused by the database. | Every box of the event is published the same way. | The select is disabled, with the reason ("Locked: voting has opened in …"). |
| Change the contest encryption, delegated or weighted voting policy, or the bulletin board, after voting has opened (Seal at close) | Refused by the database. | The seal and the tally rely on them. | The selects are disabled: "With Seal at close, the seal relies on this setting." |
| Save the event for another reason after voting has opened, with one of those settings missing or empty (Seal at close) | Saved. | A missing or empty setting counts as its default, so its effective value didn't change. | Nothing special. |
| Stop one channel while another enabled channel is still open or never started | Not sealed yet. | That channel could still take votes. Nothing is guessed on your behalf. | Stop dialog: "… *channel* is still enabled and not closed". Card header: "*Channel* is enabled and not closed: stop it to seal the ballot boxes." |
| Early Voting enabled, never started, Online started and then closed | Not sealed yet: Early Voting holds the seal, like any enabled channel. | The platform doesn't decide for you that a channel is over. | Stop dialog and card header name Early Voting. Stop it (it closes without opening) to seal the boxes. |
| A channel the election doesn't enable is Open or Paused, or ran (for example, unchecked in the election's channels after it took ballots) | Not sealed yet: it holds the seal until it is Closed. Its close counts for the close time. | It may have taken ballots, or still take them. | Stop dialog and card: "*Channel* is open but not enabled for this Post: enable it again and stop it to seal the ballot boxes." Enable it again in the election's settings, then stop it. |
| A channel the election doesn't enable that never started | It doesn't count. | It took no ballots. | Nothing. |
| The ballot box holds a ballot of a channel that isn't Closed (for example, after the channel's status was changed outside the Admin Portal) | Not sealed yet; the sealer retries every minute. | The ballots are evidence that the channel took votes. | Sealing overdue: "*Channel* has ballots in this ballot box and isn't closed: stop it to seal the ballot box." |
| A ballot's channel is none of Online, Kiosk, Early Voting or Telephone | Seal stopped; the box stays locked; a `BallotBoxSealFailed` ERROR entry is logged. | The stored data was altered or corrupted. It is an incident, not a retry. | Not sealed: incident, "A ballot has an unknown voting channel." |
| Stop a channel that never started (Seal at close) | It closes without ever opening, and stops holding the seal. Its close doesn't move the close time. | Otherwise it would hold the seal forever. | Stop dialog: "*channel* never opened: stopping it means it won't open." |
| Scheduled close or signed Close voting request that names an enabled channel that never started, while another enabled channel of the election ran (Seal at close) | It closes that channel too. | Same. | The channel shows Closed. |
| Scheduled end of Early Voting while Online never started | Only Early Voting closes. Online stays able to open; the boxes are sealed after Online closes. | A schedule closes only the channels it names. | Online stays *Not started*. |
| Scheduled close of an election that never opened (for example, its opening waited for initialization) | Nothing changes. | Closing it would seal empty boxes and stop it from ever opening. | The election stays *Not started*; the log says "Nothing to close at *election* on schedule: it never opened, so it stays as it is". |
| Event-wide **Stop Voting** while some elections never opened | Those elections close too, and their empty ballot boxes are sealed. | A manual stop does what the administrator asks. | The Stop dialog names them: "*names* never opened: stopping closes them and seals their empty ballot boxes." |
| Scheduled or signed close names a channel the election doesn't enable | That channel is left alone. | A close only changes the channels the election uses. | Nothing changes for that channel. |
| Pause Voting | Not sealed. | Paused is not closed. | Open. |
| All channels that count closed, no grace period applies | Sealed at the deadline, normally within moments. | The deadline is the close time. | Sealing now, then Sealed, publishing, then Sealed. |
| All channels that count closed, Online ran, grace period of *N* minutes | Eligible late casts are accepted until the deadline; then sealed. This holds also if the election no longer enables Online. | The grace period lets voters who signed in before the close finish. | Sealing at *close + N minutes*. |
| Online enabled with a grace period, but closed without ever opening | No grace period: the deadline is the close time. | No online voter can be finishing a ballot. | Sealing now. |
| A cast after the seal deadline, while an earlier end date plus grace would still allow it | Refused. | The seal deadline is the one the seal records. | The voter gets the closed error. |
| A cast arrives while the box is being sealed | It waits, then is refused. | The seal holds the box's lock. | The voter gets the closed error. |
| A cast was already being saved when the seal started | It finishes first and is included in the seal. | The seal waits for it. | It is in the counts. |
| Deadline passed, Datafix votes still in progress | The seal waits and retries every minute. | Sealing an unresolved vote would fix the wrong outcome. | Sealing overdue: "*N* votes are in progress in Datafix …". |
| Deadline passed, the attempt fails (for example, the bulletin board or the voter list can't be reached, a ballot can't be read, or a configuration doesn't parse) | Retried every minute; not an incident. | The cause may be temporary. | Sealing overdue, with a text for the kind of error ("The last attempt couldn't …; it is retried every minute."). The details are in the service log. |
| Deadline passed, no attempt for 3 minutes | Nothing seals the box until the sealer runs. | The Beat scheduler or the seal worker is not running. | Sealing overdue: "Not tried yet" or "Last tried at …: … Check Beat and the seal worker." |
| Sealed, but posting to the bulletin board or uploading the record fails | The box stays sealed and locked; publication is retried every minute. | Publication is idempotent; the lock does not depend on it. | Sealed, publishing; Seal record "Not yet"; the election cannot be tallied yet. |
| A stored ballot does not match its Ballot ID, or has no content or Ballot ID | Seal stopped; the box stays locked; a `BallotBoxSealFailed` ERROR entry is logged. | The stored data was altered or corrupted. It is an incident, not a retry. | Not sealed: incident, with the reason; the incident banner. |
| The event has no bulletin board | Seal stopped; the box stays locked. | There is nowhere to post the seal. | Not sealed: incident, "The election event has no bulletin board." |
| The bulletin board already has a seal for this box (for example after restoring an older database backup) | Seal stopped; the box stays locked; a `BallotBoxSealFailed` ERROR entry is logged. No second seal is posted. | Exactly one seal per ballot box. | Not sealed: incident, "A seal for this ballot box is already on the bulletin board." |
| Start Voting on a closed election or event | Refused, before or after the seal. | Closed is final with Seal at close. | "Voting can't start again: …"; Start isn't offered for elections with seals. |
| Start or resume the whole event while some elections are closed | The other elections open; the closed ones stay closed. | Closed is final per election. | The Start warning and the **Some elections stay closed** alert. |
| Voter edited, disabled, moved or released after the seal | User saved; ballots in sealed boxes kept, others discarded; the sealed count is unchanged. | Eligibility was fixed at the seal. | A warning in the server log. |
| Delete an election or event that has seals | Refused, before anything is deleted. | Seals are permanent evidence. | The Admin Portal checks first and explains; the server and the database refuse any other path. |
| A publication after the close adds an area | No seal for the new area; the tally stays refused. | The set of boxes is fixed at the close. | See the runbook. |
| A sealed box's area loses the election's contests | The tally is refused. | A sealed box must not be dropped from the count. | "The sealed ballot box of … is not in this tally …" |
| Create a tally while a box is not Sealed | Refused. | The tally counts only published seals. | The election's checkbox is disabled, with the reason under the list. |
| Tally finds the stored ballots differ from the seal | That election's tally stops; `TallyBallotBoxRejected` is logged. | The ballots were changed after the seal. | "The ballot box of *election*, *area* does not match its seal". |
| Tally finds duplicate copies of the seal entry, or entries not signed by the event's key | Copies count once; unsigned entries are ignored with a warning. | Only a validly signed entry can be a seal. | The tally proceeds. |
| Tally finds a published seal but no validly signed entry on the board, two distinct validly signed entries, or a board entry for a box whose stored seal is missing or not published | That election's tally stops; `TallyBallotBoxRejected` says what differs. | The log and the database disagree, or the key signed a second seal. | "The ballot box of *election*, *area* does not match its seal". |
| The policy is switched off in the database after a seal | The tally still verifies every box of the election. | A seal on the bulletin board decides, not the setting. | As for Seal at close. |
| Initialization report | Not gated, no seal check. | It runs before voting. | Unchanged. |
| Tally finds a match | Counts from the manifest; `TallyBallotBoxVerified` is logged once. | | The tally proceeds. |

## Limits

- **The signing key.** The seal is signed by the election event's protocol
  manager key, the key that signs every electoral log entry. It is kept in the
  database, encrypted under the deployment's master secret. Observers compare
  its fingerprint with one obtained from the organization before the close.
  See
  [which key to trust](../../../04-election_auditors/02-tutorials/06-auditor_verify-ballot-box-seal.md#which-key-to-trust).
- **Database superusers.** The lock is a database guard. A superuser, or the
  role that owns the tables, can bypass it, and can also change what the tally
  checks against. See
  [what a superuser can still do](../../../05-reference/05-cryptography_security/02-security/03-ballot-box-seal-database-role.md#what-a-superuser-can-still-do).
- **Before the close.** The seal fixes the box at the close. For evidence
  about the time before the close, compare the seal's Ballot IDs with the
  `CastVote` log entries (`--cast-votes` in the verify command).
- **Privacy of dispositions.** The record lists, for each Ballot ID, how it
  counts (counted, replaced, not eligible, discarded), its weight and its
  channel. Anyone who has the record and a voter's receipt can see whether that
  ballot was replaced by a later one, and a weight may identify a voter in
  weighted or delegated voting. With **Restricted** (the default) only the
  people the organizers give the record to can do this; with **Public**,
  anyone. Deployments that rely on revoting against coercion should weigh this.
  The electoral log already links a voter's ballots for anyone who can read
  it; see
  [what the record reveals](../../../04-election_auditors/02-tutorials/06-auditor_verify-ballot-box-seal.md#privacy-what-the-record-reveals).
