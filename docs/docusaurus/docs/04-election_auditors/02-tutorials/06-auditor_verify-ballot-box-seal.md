---
id: auditor_verify-ballot-box-seal
title: Verify a ballot box seal
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

When an election event uses the **Seal at close** policy, every ballot box (the
ballots of one election in one area) gets a **seal record** when voting
closes. This guide shows how to check a seal record on your own computer,
without access to the voting system. See
[Ballot box seal](../../02-election_managers/02-reference/02-election-event/20-ballot-box-seal.md)
for how sealing works.

You need:

- the seal record, a JSON file;
- the `step-cli` binary;
- the fingerprint of the key you trust, obtained from the organization
  **before voting closes** (see [Which key to trust](#which-key-to-trust));
- optionally, a Ballot ID from a voter's receipt, and an `export-cast-votes` CSV.

## Get the seal record

Each record is a JSON file. Where it is depends on the event's **Seal Record
Publication** policy, which the organization chooses before voting opens and
can't change afterwards
([Seal Record Publication policy](../../02-election_managers/02-reference/02-election-event/20-ballot-box-seal.md#the-seal-record-publication-policy)):

- **Restricted** (the default): the record is a private document of the
  election event. An administrator with the document download permission
  downloads it with the **Seal record** button on the election's Ballot boxes
  card (Dashboard), and gives it to you. Ask the organization how it shares the
  records, and for the key fingerprint at the same time.
- **Public**: the record is in the election event's public documents bucket,
  where published results go too, at a stable path. Anyone with the tenant,
  event, election and area IDs can download it without signing in:

  ```text
  tenant-<tenant id>/event-<event id>/ballot-box-seals/<election id>/<area id>.json
  ```

  Administrators also open it from the **Seal record** link on the Ballot
  boxes card.

Under both policies the file is the same, and the check below is the same: the
record proves itself through the signed log entry and the key you trust, not
through where you got it. Keep a copy: a record you downloaded at the close
lets you detect a later change.

## What the seal record contains

| Field | Content |
| --- | --- |
| `format` | `step/ballot-box-seal/v1` |
| `tenant_id` | The tenant ID. |
| `election_event`, `election`, `area` | Each an `id` and a `name`. The IDs are signed. The election and area names are those stored when the box was sealed (for seals made before the upgrade, see [below](#seals-made-before-the-upgrade)); the command checks them against the signed statement. The event name is not signed. |
| `closed_at` | When voting closed (the latest close of the election's channels that ran), UTC, in the form `2026-10-06T19:22:16Z`. |
| `grace_deadline` | When the grace period ended (equal to `closed_at` without one). |
| `sealed_at` | When the box was sealed. |
| `eligible_voters` | The number of voters eligible in this area for this election at the seal. |
| `ballots` | `in_the_box` (every ballot, in any status) and `counted` (the number of counted ballots). |
| `close_request` | The signed Close voting request, if there was one: its `id`, `signing_code`, and each signer's `name` and `certificate_sha256`. `null` for Stop Voting without signatures and for the scheduled close. Only the request ID is signed. |
| `seal_hash` | SHA-512 of the manifest, hex. |
| `log_entry` | The `id` of the `BallotBoxSealed` entry on the electoral log, and its `statement_kind`. **Not signed.** |
| `system_public_key` | The public key that signed the entry (base64 SPKI DER), taken from the signed message. |
| `message` | The signed log message, as readable JSON. |
| `message_b64` | The same message as the exact bytes the log stores (base64). |
| `entries` | One entry per ballot, sorted: `ballot_hash` (SHA-512 of the stored encrypted ballot, hex), `ballot_id`, `disposition` (`counted`, `replaced`, `not-eligible` or `discarded`), `weight` and `channel`. |
| `manifest` | The exact manifest bytes (base64). The seal hash is computed over these bytes. |

The record holds **no** voter IDs, pseudonyms, times of individual ballots or
network data. Each entry carries the hash of the encrypted ballot, not the
ballot itself. See
[Privacy: what the record reveals](#privacy-what-the-record-reveals) for what the
entries do reveal.

## Run the check

```bash
step-cli step verify-ballot-box-seal <record.json> \
  [--expected-key <base64-key|fingerprint>] \
  [--election-id <id>] [--area-id <id>] \
  [--ballot-id <id>] \
  [--cast-votes <export.csv>]
```

| Option | Effect |
| --- | --- |
| `<record.json>` | The seal record to check. Required. |
| `--expected-key` | The key you trust: the base64 public key, or its 128-bit fingerprint (32 hex digits; spaces, colons and dashes are ignored). A different key makes the check fail. |
| `--election-id`, `--area-id` | The ballot box you expect. A record signed for another election or area fails. Use them so that a genuine record of another box can't be passed off as this one. |
| `--ballot-id` | Reports whether this Ballot ID is in the ballot box and how it counts. |
| `--cast-votes` | Compares the box's Ballot IDs with the `CastVote` entries of an `export-cast-votes` CSV (see [below](#what---cast-votes-proves)). |

The examples below are real output from a test event: one election with no
grace period, an area with four ballots (one of them replaced by the voter's
later ballot), closed with Stop Voting and no signatures. IDs and names are
the test event's.

**A valid record, checked against a trusted key and the expected box**, with a
Ballot ID and the CastVote export:

```text
$ step-cli step verify-ballot-box-seal j1_record_area_A.json --expected-key '1EE6 1784 F39B 5B7D 0768 69F5 E3F3 E7C8' --election-id ad2eff27-72e1-42ef-aad7-bac61a1438ba --area-id f84aa227-9a4d-4c15-831f-49a1ba804b4a --ballot-id 4918c9c58f6fea195877ade1f00a944eb99870ade68b71333923903e145ac950 --cast-votes j1_cast_votes.csv
Ballot box    E2E main election, E2E area A (names as in the signed statement)
Event         VOTE-FREEZE seal journey b830121b (name not signed), ID e8e036a5-7c67-47aa-8062-9d500ee64ce0
IDs           election ad2eff27-72e1-42ef-aad7-bac61a1438ba, area f84aa227-9a4d-4c15-831f-49a1ba804b4a
Closed at     2026-10-06 19:22:16 UTC, grace period until 2026-10-06 19:22:16 UTC
Sealed at     2026-10-06 19:22:17 UTC
Ballots       4 in the box, 3 counted, 4 eligible voters

✓ Entries            4 entries, sorted by ballot hash; weights and counts follow the seal rules
✓ Manifest           rebuilt from the header and entries: the same bytes as the record's
✓ Seal hash          the SHA-512 of the manifest is the hash in the signed statement:
                     30438ad2292f59b57194e32f6e2415d7dc470e89287059fa1f160c8a9168fb5290179d8ff0461c85acbbbb3c02f08ef061f753bb81667f9ff2669fa5b3672cef
✓ Statement          names this election, area, counts and close request
✓ Names              E2E main election, E2E area A: the names in the signed statement
✓ System signature   Ed25519, election event key 1EE6 1784 F39B 5B7D 0768 69F5 E3F3 E7C8
✓ Expected key       1EE6 1784 F39B 5B7D 0768 69F5 E3F3 E7C8 is the key you gave
✓ Sender signature   a system entry: the sender is the same key
✓ Time               the signed statement says 2026-10-06T19:22:17Z; closed, grace deadline and seal in order
✓ Close request      none: closed without a signed Close voting request
- Log entry          #11 (not signed): check that the bulletin board's BallotBoxSealed entry for this box has this seal hash
✓ Expected box       election ad2eff27-72e1-42ef-aad7-bac61a1438ba, area f84aa227-9a4d-4c15-831f-49a1ba804b4a: the one you gave
✓ Ballot ID 4918c9c58f6fea195877ade1f00a944eb99870ade68b71333923903e145ac950 in the ballot box: counted
✓ CastVote entries   4 in j1_cast_votes.csv; sealed ballots without an entry: 0; entries without a sealed ballot: 0

Seal valid
```

The exit status is 0. After a close signed by a quorum, the Close request line
shows the request instead: its ID, signing code and number of certificate
signatures, marked "(signing code and signers not signed)".

**The same record without `--expected-key`.** The command cannot tell whether
the key is one you trust, so it says so on the signature line, marks the
Expected key line with `!`, and names the key in the verdict. The exit status is
still 0:

```text
$ step-cli step verify-ballot-box-seal j1_record_area_A.json
…
✓ System signature   Ed25519, signed by the record's key 1EE6 1784 F39B 5B7D 0768 69F5 E3F3 E7C8
! Expected key       not checked (no --expected-key): compare 1EE6 1784 F39B 5B7D 0768 69F5 E3F3 E7C8 with the key you trust
…
Seal valid for key 1EE6 1784 F39B 5B7D 0768 69F5 E3F3 E7C8 (not checked against a key you trust: pass --expected-key)
```

**A tampered record**: a copy with one entry removed. The verification stops at
the first failure and prints only that one. Nothing from the record is shown as
verified:

```text
$ step-cli step verify-ballot-box-seal j1_record_A_tampered_dropped_entry.json --expected-key '1EE6 1784 F39B 5B7D 0768 69F5 E3F3 E7C8'
Record        j1_record_A_tampered_dropped_entry.json: election ad2eff27-72e1-42ef-aad7-bac61a1438ba, area f84aa227-9a4d-4c15-831f-49a1ba804b4a (not verified)

✗ Manifest           the manifest rebuilt from the header and entries differs from the record's
- Other checks       not reported: the verification stops at the first failure

Seal NOT valid
Error! the seal is not valid: Manifest: the manifest rebuilt from the header and entries differs from the record's
```

Other failures look the same, with their own line:

- another key: `✗ Expected key       signed by key …, not the expected key`;
- another box: `✗ Expected box       the seal is for election …, area …, not the
  expected area …`;
- names that are not those of the signed statement: `✗ Names` with both;
- a time not in the canonical form (UTC, whole seconds, ending in `Z`), times
  out of order, more counted ballots than eligible voters, or a counted ballot
  with a weight below 1: a failed Manifest, Time or Entries line.

If the seal is valid but a check you asked for fails (the Ballot ID is not in
the box, or the CastVote comparison differs), the verdict ends with ", but a
requested check failed (marked ✗ above)". In every failure case the command
exits with a non-zero status, so you can use it in scripts.

### Seals made before the upgrade

Seals made before the platform's migration `1791000001700` didn't store the
election and area names. Their records carry the names as they were when the
record was published. If an election or area was renamed between the seal and
the publication, the command reports a genuine seal as **Seal NOT valid**,
with a failed Names line:

```text
✗ Names              the record says <new election name>, <new area name>; the signed statement says "Ballot box of <old election name>, <old area name> sealed: …"
```

To tell such a record from a tampered one:

1. Check that every other line is ✓: the names check runs after all the
   cryptographic checks, so they are all printed.
2. Run it with `--election-id` and `--area-id`: the IDs are signed, so the
   Expected box line confirms the ballot box.
3. Read the signed description in the Names line: it holds the names at the
   time of the seal. Confirm with the organization that the election or area
   was renamed from those names.

If all three hold, the seal is genuine and only its readable names are newer.

## What each line proves

| Line | Check | A failure means |
| --- | --- | --- |
| Entries | The entries are well formed and sorted; only counted entries have a weight, of at least 1; no more counted ballots than eligible voters. | The record was edited or truncated. |
| Manifest | The manifest rebuilt from the record's header (IDs, times, close request ID, eligible voters) and entries is byte-for-byte the record's manifest. Times must be in the canonical form. | The readable fields and the hashed bytes disagree: an entry, a time or an ID was changed, added or removed. |
| Message | The readable `message` is the same as `message_b64`, and it is a `BallotBoxSealed` statement. | The readable message was edited. (Shown only on failure.) |
| Seal hash | SHA-512 of the manifest equals the hash in the signed statement. The full hash is printed. | The manifest is not the one that was sealed. |
| Statement | The signed statement names the same election, area, counts and close request as the manifest. | The record mixes data from different boxes. |
| Names | The record's election and area names give back the signed statement's description. | The names were changed. |
| System signature | The signature over the statement verifies with the record's system key. The command prints the key's fingerprint: the first 16 bytes of the SHA-256 of the key, in eight groups of four hex digits. | The statement was changed, or it was signed by another key. |
| Expected key | The record's key is the one you gave with `--expected-key`. | The record was not signed by the key you trust. |
| Sender signature | The statement is a system entry: its sender is the system key itself. | The entry did not come from the system. A record signed by any other sender is "Seal NOT valid". |
| Time | The seal time in the signed statement equals the manifest's, and close time ≤ grace deadline ≤ seal time. | The statement and the manifest disagree on the seal time, or the times are out of order. |
| Close request | The statement's Close voting request is the manifest's. The signing code and signers are shown, but they are not signed. | The statement and the manifest disagree on the close request. |
| Log entry | Not checked: the log entry ID is not signed. | |
| Expected box | With `--election-id` or `--area-id`, the signed IDs are the ones you gave. | The record is for another ballot box. |

What a valid seal does **not** prove on its own:

- **that you trust the key**: compare the fingerprint (next section);
- **that this seal is the one on the bulletin board.** Anyone holding the key
  could sign a second record for the same box, and it would pass this command.
  Compare the full seal hash with the `BallotBoxSealed` entry for this box in
  the event's Logs tab or its export, and with the copies other observers kept.
  The log entry ID in the record is not signed;
- the event name, and the signers' names and signing code: they are not signed;
- that the eligibility decisions (`not-eligible`, weights) were right; the seal
  fixes them, it does not justify them;
- that the encrypted ballots match those mixed and decrypted in the tally; that
  link is the tally's own verification.

## Which key to trust

The seal is signed by the **election event's protocol manager key**. It is the
same key that signs, as the system, every entry of the event's electoral log:
the voting period closes, the `CastVote` entries and so on. No new key is
created for the seal.

**Where the key is kept.** The key is stored in the platform's database, in its
secret store, encrypted under the deployment's master secret. Nothing outside
the database anchors it today:

- The trustees' signed Braid configuration holds the protocol manager key of
  each **election's** board, not of the event's board, so you cannot check the
  seal key against the trustee-signed configuration.
- Someone with superuser access to the database can point the event at another
  bulletin board and key, and have a new seal signed for a changed ballot box.
  The tally would then check against that board and key. What still catches it
  is the original `BallotBoxSealed` entry, which stays on the original board,
  the copies of records and seal hashes kept by observers, and a check with
  `--expected-key` using a key obtained before the change. See
  [what a database superuser can still do](../../05-reference/05-cryptography_security/02-security/03-ballot-box-seal-database-role.md#what-a-superuser-can-still-do).

So:

1. **Before voting closes**, get the event key's fingerprint (or full base64
   key) from the organization, through a channel you trust. You can cross-check
   it with the system key of other log entries of the same event that you
   already hold, such as the `CastVote` entries.
2. Run the command with `--expected-key <fingerprint>`, and with
   `--election-id` and `--area-id`.
3. If the key differs, the record was not signed by the key you were given.
   Treat it as invalid and report it.
4. Compare the seal hash with the bulletin board and with other observers'
   copies (see above).

The fingerprint is 128 bits. Passing the full base64 key compares every byte.

## Look up a Ballot ID

The Ballot ID on a voter's receipt, in the `export-cast-votes` CSV and in the
seal record are the same 64 hex characters. Pass it in full; case does not
matter. The command says how it counts:

| Result | Meaning |
| --- | --- |
| `in the ballot box: counted` | The voter's latest valid ballot, counted. A weight other than 1 (delegated or weighted voting) is shown: `counted (weight 3)`. |
| `in the ballot box: replaced by a later ballot, not counted` | The voter cast a later ballot, which counts instead. |
| `in the ballot box: not eligible, not counted` | The voter was not enabled for this area and election at the seal. |
| `in the ballot box: discarded, not counted` | The ballot was discarded before the close (for example, by Datafix). |
| `not in the ballot box` (✗) | The Ballot ID is not in this box. Check that you have the right election and area. |

## What `--cast-votes` proves

Each cast also writes a signed `CastVote` entry to the electoral log, with the
ballot hash, election, area and Ballot ID. The `step-cli step
export-cast-votes` command exports those entries to a CSV file. It reads the
log directly, so it needs read access to the log.

With `--cast-votes`, the command keeps the CSV rows of this box's election and
area, and compares their **Ballot IDs** with every sealed ballot, in any
disposition. It reports, and lists up to five Ballot IDs of each:

- **sealed ballots without a CastVote entry**: a ballot in the box that never
  went through the cast path, for example one inserted directly in the
  database before the close;
- **CastVote entries without a sealed ballot**: a ballot that was cast but is
  not in the box, for example one deleted before the close.

Both counts at 0 show that the box holds the same Ballot IDs as were cast. It
says nothing about how each one counts: a status change (valid to discarded), a
change that makes another of the voter's ballots the latest one, an eligibility
change or a channel change before the seal would not show here.

Know where the CSV comes from:

- `export-cast-votes` does not check the entries' signatures, and the area of
  each row comes from an unsigned column of the log. Export it yourself, from a
  log you can read, rather than taking a file from the party you are checking.
- The `CastVote` entry is written after the cast is saved, best effort. A
  missing entry can also come from a delivery failure.
- A CSV exported from another event, or before the close, has no or fewer rows
  for this box, and every sealed ballot then shows as without an entry. Check
  the row count on the line first.

Investigate any non-zero count before drawing a conclusion.

## Privacy: what the record reveals

The record pairs each Ballot ID with how it counts, its weight and its channel.
Compare it with what other sources already show.

**What the electoral log already shows.** Each `CastVote` entry of the
electoral log carries the voter's pseudonym, the ballot's **channel** (Online,
Kiosk, Early Voting or Telephone), and the voter's **IP address** and
**country**. The pseudonym is an unsalted hash of the voter's ID
(`hash_voter_id`), the same for every ballot of that voter, so anyone who knows
a voter's ID can find that voter's entries. Anyone who can read the log can
link a voter's ballots, see that the voter voted again, and, with a Ballot ID
from a receipt, find that voter's other ballots. Revoting and channels are
visible there, with or without the seal record.

**What the ballot locator shows.** When the event's `show_cast_vote_logs`
setting is **ShowLogsTab**, the voting portal's ballot locator lists every
`CastVote` entry of the election, with its log message, to any voter signed in
to that election, for example during voting.

**What the seal record adds**, for each Ballot ID:

- whether it was counted as **not eligible** (the census at the seal);
- its **weight**, which in weighted or delegated voting may identify a voter;
- with the **Public** policy only: access **without signing in**, to anyone
  with the event's IDs, and **after the close**, for as long as the file is
  kept.

**Restricted** (the default) keeps the record to the people who receive it from
the organizers. Organizations that rely on revoting against coercion, or on the
secrecy of eligibility or weights, should choose **Restricted** and decide whom to
give the records to; the log and the locator setting need the same care.
