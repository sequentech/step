---
id: election_verifier
title: Verifying the Result with the Election Verifier
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The election verifier is the part of the TOE that checks the counting of an
election event independently of the running system. It needs no key, no account and
no network connection. It checks the public record of the election event:

- the key generation by the trustees, with every trustee's proofs and the
  joint election key;
- the proof attached to every encrypted ballot that entered the counting;
- the order of the mixing and every shuffle proof;
- the decryption by exactly the threshold of trustees, with their proofs;
- that the decrypted votes published by the TOE equal its own decryption.

The election verifier runs on demand: each run checks the file it is given and
ends when the check is done. It is not a service that keeps running.

The election board runs it after downloading the verification data, before the
result is announced; it needs no permission in the TOE. Observers and voters
can run it on the data that the election organizer publishes.

## What you need

- The verification data for each batch of the completed tally: the files that
  the election board downloads as
  [Downloading the verification data](#downloading-the-verification-data) describes,
  or the copies the election
  organizer publishes. One file contains one protocol board's batch and its key
  ceremony evidence; a tally can have several batches.
- The full values "Configuration hash" and "Tally ID" of each file, obtained
  independently of that file: from the election organizer's publication,
  read from the board's minutes, or confirmed by the election board. Keep each
  file paired with its own anchors.
- The image of the election verifier from the accepted TOE delivery
  ([Verifying the Delivery](../../05-reference/08-delivery_verification.md)).
- A computer with a container runtime, the verification workstation. A board member
  uses one that the election organizer controls; observers and voters use their own.
  To check a ballot
  ([Checking that a ballot was counted](#checking-that-a-ballot-was-counted)), it
  also needs the JSON reader `jq`.

## Downloading the verification data

In the post-processing phase, an election board member opens the completed
results tally from the election event's tab "Tally". Before that phase the TOE
refuses the download, and the admin portal shows the message "Verification data
can be downloaded once post-processing has started.". The download does not
export initialization reports or key-ceremony-only data, and the TOE records each
one
([Downloading the verification data](08-evaluation-and-post-processing.md#downloading-the-verification-data)).
In the section "Election verification", for every listed protocol board:

1. Record its board name and the full values "Configuration hash" and "Tally ID" in
   the board's minutes. The TOE shows these values separately from the file.
2. Select the button "Download verification data" in that same protocol board's section. Keep the
   downloaded file `verification-<protocol-board UUID>.json` paired with the values
   recorded in step 1. Downloading the file does not verify it.
3. Repeat for every listed batch. Do not substitute another batch's identifier,
   even when the batches use the same configuration hash.

The displayed value "Tally ID" is the protocol board's identifier written as an
unsigned decimal number with up to 39 digits. It is **not** the tally-session
ID in the election event's tally list or URL. Copy the complete displayed value as text;
do not round it in a spreadsheet or convert it through a floating-point
number.

If a completed results tally has no listed protocol boards, or its anchors
or download cannot be loaded, stop and report the problem. Do not substitute a
file or identifier from an earlier tally.

## Running the election verifier

Put a verification data file in a directory of its own and run the image on
it with that file's independently recorded anchors, without network access
and with the directory read-only:

```sh
podman run --rm --network none -v <directory>:/data:ro <election-verifier image> \
  --transcript /data/<verification data file> \
  --configuration-hash <configuration hash> \
  --tally-id <Tally ID>
```

The command `docker run` takes the same options. The image runs the verification of the
trustee tool (`trustee verify`) with the options you append:

| Option | Value |
|---|---|
| `--transcript` | The verification data file. |
| `--configuration-hash` | The full configuration hash recorded for this batch: 128 lowercase hexadecimal digits. Required. |
| `--tally-id` | The protocol-board Tally ID for this file, copied from the value "Tally ID" as an unsigned decimal number. The tool also accepts the corresponding protocol-board UUID; the tally-session UUID is not this identifier. Required for every file downloaded by this results-tally procedure: without it the status is `invalid` and the field `detail` reads "tally evidence requires an independently supplied tally identifier", and a different identifier makes the check fail. |
| `--ballots-hash` | Optional: the hash of the batch of encrypted ballots that entered the counting, if the election organizer publishes it. A different batch makes the check fail. |
| `--ballot-ciphertext` | Optional, may be repeated: an encrypted ballot to look for among the counted ballots (see [Checking that a ballot was counted](#checking-that-a-ballot-was-counted)). |

Run this command separately for every batch, using each file's own anchors.
A board member keeps the reports and exit codes with the board's minutes. The complete tally is
not verified until every expected batch has status `verified`; a successful
check of one file does not cover the others.

## Reading the result

The tool prints a report in JSON and ends with exit code 0 only if the status is
`verified`.

| Field | Meaning |
|---|---|
| `status` | `verified`, `incomplete` or `invalid` |
| `detail` | What was verified, or why not |
| `configuration_hash`, `tally_id` | The configuration hash and the Tally ID you supplied; `tally_id` is shown as a decimal number |
| `public_key_hash` | The hash of the election key that the trustees generated |
| `ballot_count` | The number of encrypted ballots that entered the counting |
| `plaintexts` | The decrypted votes in their encoded form, in the final order of the mixing |
| `ballot_inclusions` | For each `--ballot-ciphertext`, its hash and how often it occurs among the counted ballots |

| Status | Exit code | What it means | What to do |
|---|---|---|---|
| `verified` | 0 | Every check passed. With the counting, the field `detail` reads "DKG, ballot proofs, ordered shuffles, endorsements, threshold decryption and plaintexts verified". | Compare the field `ballot_count` with the number of votes that the result states as counted for the contest and area of this batch. |
| `incomplete` | not 0 | The record ends before the counting is complete. The tool ends with the error "public verification incomplete: …". | Check that you have the file of the completed tally. If it persists, report it to the election board. |
| `invalid` | not 0 | A check failed. The field `detail` says which, for example "invalid shuffle proof", "published plaintexts differ from independently verified decryption" or "ballots do not match the trusted batch hash", and the tool ends with the error "public verification failed: …". | Report it at once to the election board, with the report. The election board treats it as a security-relevant event ([Events in the post-processing phase](11-security-relevant-events.md#events-in-the-post-processing-phase)). |

The election verifier does not apply the counting rules: it shows that the
decrypted votes are exactly the votes that were cast and mixed, not that the
result adds them up correctly. Compare the result with the electoral
regulation's counting rules as a separate check. It also does not check the
encrypted key shares that the trustees exchanged; each trustee's instance of
the trustee server checks those.

:::warning
Take the configuration hash and the Tally ID only from a source independent of
the verification data file. A file, hash and Tally ID that come from the same
place prove nothing if that place has been manipulated.
:::

## Checking that a ballot was counted

A voter who copied their encrypted ballot from the ballot locator of the voting
portal (the button "Locate your ballot" of the screen "Ballot list") during the execution
phase can check that it was counted. The voter saves the encrypted ballot, exactly
as the ballot locator shows it, to a text file. This is the complete JSON ballot, including its signatures and the
encrypted value in the list `contests`. The native `--ballot-ciphertext` option takes
that encrypted value, not the surrounding JSON. If the
voter voted again, use only the copy for the last vote cast: earlier votes are
not counted. Checking an earlier ciphertext can therefore report it absent
without indicating a fault in the tally. Obtain the final ciphertext before
the ballot locator closes.

Keep the original file unchanged. On the verification workstation, use the
JSON reader `jq` to extract the single contest into a separate file. In this
example, the complete saved ballot is named `ballot.json`:

```sh
jq -er '.contests | select(type == "array" and length == 1) | .[0] | strings' ballot.json > ballot-ciphertext.txt
```

The command must finish successfully and create a nonempty file containing one
encoded value. Stop if it prints an error or the file is empty; do not choose
one contest from a ballot containing several contests, delete fields, or alter
the saved ballot. This procedure covers a ballot whose list `contests` holds one
encrypted value: an election with one contest, or one whose setting "Contest
encryption policy" is "Multiple Contests". The evaluated configuration does not fix
this: the policy comes with the approved package
([Settings of the election event](../04-administrator_handbook/05-tenant_and_event_settings.md#settings-of-the-election-event)).
For any other ballot this check is not available, and the election organizer tells
the voters so
([Publishing the verification data](#publishing-the-verification-data)).
`jq` is an auxiliary JSON reader, not the cryptographic verifier.

Run the accepted verifier with the extracted value:

```sh
podman run --rm --network none -v <directory>:/data:ro <election-verifier image> \
  --transcript /data/<verification data file> \
  --configuration-hash <configuration hash> \
  --tally-id <Tally ID> \
  --ballot-ciphertext "$(cat ballot-ciphertext.txt)"
```

The original complete ballot remains the evidence copied from the locator.
The verifier parses the extracted ciphertext and checks its inclusion against
the independently verified tally; successful JSON extraction is not a
cryptographic verification.

Several ballots from the same batch can be checked at once by repeating
the option `--ballot-ciphertext`. If the organizer publishes several batch files and the
voter does not know which one contains their ballot, first verify every file
without this option. Then look for the ciphertext in each batch using that
batch's own configuration hash and Tally ID. Absence from one batch alone does
not mean the ballot is absent from the complete tally.

The status is `verified` only if the ballot is among the counted ballots and
every shuffle and decryption proof holds; the field `ballot_inclusions` then shows it
with at least one occurrence. If the ballot is not among them, the status is
`invalid` and the field `detail` reads "requested ballot ciphertext is absent from the verified
tally input". If it is absent from every relevant, otherwise verified batch, report
the problem to the election board. A voter who cannot run the tool can ask an
observer they trust to run it for them.

## Publishing the verification data

In the post-processing phase the election organizer, for each election event:

1. obtains every batch's verification data file and the separately recorded
   anchors from the election board, using
   [Downloading the verification data](#downloading-the-verification-data);
2. publishes every file with its board name, full configuration hash and Tally
   ID on a public channel that it names to the voters, for example its own
   website; make the file-to-batch pairing explicit;
3. publishes each batch's configuration hash and Tally ID a second time on
   another channel, for example the board's published minutes or an official
   notice, so that voters and observers can take them independently of the
   files;
4. makes the election verifier available to voters and observers;
5. tells the voters, in its own words, where the data is and how to check
   their ballot with the command in
   [Checking that a ballot was counted](#checking-that-a-ballot-was-counted)
   (the options `--transcript`, `--configuration-hash`, `--tally-id` and
   `--ballot-ciphertext` with the encrypted ballot they saved), or whom to ask
   to run it for them, or that this check is not available for their ballot.

The verification data contains no names and no link between voters and votes.
It contains the encrypted ballots and the decrypted votes in mixed order.
