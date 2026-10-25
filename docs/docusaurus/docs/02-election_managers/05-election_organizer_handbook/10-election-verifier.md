---
id: election_verifier
title: Verifying the Result with the Election Verifier
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The election-verifier is the part of the TOE that checks the counting of an
election independently of the running system. It needs no key, no account and
no network connection. It checks the public record of the election:

- the key generation by the trustees, with every trustee's proofs and the
  joint election key;
- the proof attached to every encrypted ballot that entered the counting;
- the order of the mixing and every shuffle proof;
- the decryption by exactly the threshold of trustees, with their proofs;
- that the decrypted votes published by the TOE equal its own decryption.

The election-verifier runs on demand: each run checks the file it is given and
ends when the check is done. It is not a service that keeps running.

The election board runs it in the post-processing phase, after downloading
the verification data; it needs no permission in the TOE. Observers and voters
can run it on the data that the election organizer publishes.

## What you need

- The verification data of the election: the file that the election board
  downloads on the Tally tab with "Download verification data", or the copy
  the election organizer publishes.
- The configuration hash and the Tally ID of the election ("Configuration
  hash" and "Tally ID" on the Tally tab), obtained independently of that file:
  from the election organizer's publication, read from the board's minutes,
  or confirmed by the election board.
- The election-verifier image from the accepted TOE delivery (the Deployment
  Guide describes how the delivery is checked), and a computer with a container
  runtime.

## Running the election-verifier

Put the verification data file in a directory of its own and run the image on
it, without network access and with the directory read-only:

```sh
podman run --rm --network none -v <directory>:/data:ro <election-verifier image> \
  --transcript /data/<verification data file> \
  --configuration-hash <configuration hash> \
  --tally-id <Tally ID>
```

`docker run` takes the same options. The image runs the verification of the
trustee tool (`trustee verify`) with the options you append:

| Option | Value |
|---|---|
| `--transcript` | The verification data file. |
| `--configuration-hash` | The configuration hash of the election, as hexadecimal digits. Required. |
| `--tally-id` | The Tally ID of the election, as a UUID or as an unsigned decimal number. Required for the verification data of a counted election: without it the status is `invalid` ("tally evidence requires an independently supplied tally identifier"), and a different identifier makes the check fail. Leave it out only for an election that was not counted: its file holds no tally, `--tally-id` then gives `invalid` with the same message, and without it the verifier checks the key ceremony only ("Public DKG commitments, proofs and derived keys verified"). |
| `--ballots-hash` | Optional: the hash of the batch of encrypted ballots that entered the counting, if the election organizer publishes it. A different batch makes the check fail. |
| `--ballot-ciphertext` | Optional, may be repeated: an encrypted ballot to look for among the counted ballots (see [Checking that a ballot was counted](#checking-that-a-ballot-was-counted)). |

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
| `verified` | 0 | Every check passed. With the counting, `detail` reads "DKG, ballot proofs, ordered shuffles, endorsements, threshold decryption and plaintexts verified". | Compare `ballot_count` with the number of votes counted for the election. |
| `incomplete` | not 0 | The record ends before the counting is complete ("public verification incomplete: …"). | Check that you have the file of the completed tally. If it persists, report it to the election board. |
| `invalid` | not 0 | A check failed, for example "invalid shuffle proof", "published plaintexts differ from independently verified decryption", "ballots do not match the trusted batch hash" ("public verification failed: …"). | Report it at once to the election board, with the report. The election board treats it as a security-relevant event. |

The election-verifier does not apply the counting rules: it shows that the
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

A voter who copied their encrypted ballot in the voting portal ("Locate your
ballot") during the execution phase can check that it was counted. The voter
saves the encrypted ballot, exactly as "Locate your ballot" shows it, to a text
file, and passes the content of that file to the election-verifier. If the
voter voted again, use only the copy for the last vote cast: earlier votes are
not counted. Checking an earlier ciphertext can therefore report it absent
without indicating a fault in the tally. Obtain the final ciphertext before
the ballot locator closes.

Run:

```sh
podman run --rm --network none -v <directory>:/data:ro <election-verifier image> \
  --transcript /data/<verification data file> \
  --configuration-hash <configuration hash> \
  --tally-id <Tally ID> \
  --ballot-ciphertext "$(cat <ballot file>)"
```

The options always take this form, where the first file is the verification
data and the second the saved encrypted ballot:

```text
--transcript <file> --configuration-hash <hex> --tally-id <id> \
  --ballot-ciphertext "$(cat <file>)"
```

Several ballots can be checked at once by repeating `--ballot-ciphertext`.

The status is `verified` only if the ballot is among the counted ballots and
every shuffle and decryption proof holds; `ballot_inclusions` then shows it
with at least one occurrence. If the ballot is not among them, the status is
`invalid` with "requested ballot ciphertext is absent from the verified tally
input". A voter who cannot run the tool can ask an observer they trust to run
it for them.

## Publishing the verification data

In the post-processing phase the election organizer, for each election:

1. obtains the verification data file from the election board ("Download
   verification data" on the Tally tab);
2. publishes the file, its configuration hash and its Tally ID on a public
   channel that it names to the voters, for example its own website;
3. publishes the configuration hash and the Tally ID a second time on another
   channel, for example the board's published minutes or an official notice,
   so that voters and observers can take them independently of the file;
4. makes the election-verifier available to voters and observers;
5. tells the voters, in its own words, where the data is and how to check
   their ballot with the command in
   [Checking that a ballot was counted](#checking-that-a-ballot-was-counted)
   (`--transcript`, `--configuration-hash`, `--tally-id` and
   `--ballot-ciphertext` with the encrypted ballot they saved), or whom to ask
   to run it for them.

The verification data contains no names and no link between voters and votes.
It contains the encrypted ballots and the decrypted votes in mixed order.
