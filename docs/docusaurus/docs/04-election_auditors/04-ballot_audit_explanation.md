---
id: ballot_audit_explanation
slug: /election_auditors/ballot_audit_explanation
title: What the Ballot Audit Verifies
sidebar_position: 4
description: "How signed cast records, ballot selection, encrypted tally inputs and cryptographic proofs support verification of a tally."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# What the Ballot Audit Verifies

The ballot audit verifies the connection between recorded cast votes and the encrypted ballots that entered a tally.
The Braid verifier then checks the public key, shuffle evidence and references in the published plaintext records.
Together, these checks let election authorities and auditors trace signed cast records through the encrypted tally inputs and verified mixes.

This page explains **what** the process verifies. The
[Ballot Audit Manual](03-ballot_audit_manual/00-ballot_audit_manual.md) explains **how** to prepare the evidence, run the tools and report the outcome.

## The verification path

| Stage | What the auditor checks |
| --- | --- |
| Signed cast records | The hashed voter identifier, election and encrypted ballot agree with the signed electoral log. |
| Ballot selection | The selected ballots follow the supported revote, eligibility, weight and delegation rules. |
| Encrypted tally inputs | The correct batches contain exactly the selected ciphertexts, with the expected number of copies. |
| Cryptographic tally | The public evidence supports the key construction and shuffles. The signed plaintext records reference the same public key and final mix. |

The **electoral log** is the platform's signed record of election events, such as accepted cast votes. ImmuDB stores it.

A **ciphertext** is an encrypted value that conceals the vote choices. A **plaintext** is the decrypted value.
A **bulletin board** is the record of messages and data exchanged during the cryptographic tally. B3 stores that record.
A **batch** is a numbered group of ciphertexts on the B3 bulletin board.
A **tally session** identifies one tally, which can cover several elections, areas and contests.

## Evidence and trusted starting values

The process uses restored copies of four sources:

| Source | Evidence used |
| --- | --- |
| Application database, also called Hasura | Cast ballots, receipts, voter references, row status, timestamps, tally configuration and saved area/contest mappings. |
| Keycloak database | Voter accounts and enabled status, area and election permissions, vote weights and delegations. |
| B3 database and any separate blob files | Signed protocol messages, configuration, public key information, encrypted input batches, mixes and decryption evidence. |
| ImmuDB database | The exact bytes of signed `CastVote` messages in the electoral log. |

The election authority supplies the tally session ID, public key artifact digest and electoral-log system public key through an independently trusted channel.
An **artifact** is a piece of data attached to a protocol message, such as public key information or an encrypted ballot batch.
The board configuration, read from the authenticated B3 copy, supplies the protocol-manager and trustee keys.
The supplied public-key digest and system key anchor the checks to the authority's expected election.

Before the run, the auditor authenticates the backup digests and source capture records.
The authority states the expected elections, areas and contests, which the auditor later compares with `tally-plan.json`.
Keep the source capture records with the restore logs and `acquisition.json` produced during the run.

The backups must preserve the intended tally-time records, including **frozen eligibility**: the eligibility, weight and delegation state used for selection.
The exporter reads that state from the supplied Keycloak copy.
For PostgreSQL sources, the exporter uses read-only transactions with a stable view of each database.
The auditor uses capture records to assess consistency between the separate source snapshots.

A **digest** is a fixed-length value computed from data. SHA-512 produces a 64-byte digest.
A **pin** is an expected digest used to detect a change in those bytes.
Digests computed during the run connect one step's output to the next step's input.
They establish consistency within the run; historical authenticity depends on the independently trusted sources and capture evidence.

## 1. Confirm the tally scope

`audit-tally` creates one **scope** for each saved `tally_session_contest` row.
Each scope identifies the tenant, election event, election, area, contest, tally session, board and batch numbers.
A **tenant** is an organization's account on the platform.
Some supported ballot formats combine several contests in one encrypted value.
In that case, one scope covers every contest of that election in the area.

The exporter compares these identifiers with the saved tally configuration and mappings.
The **key ceremony** creates the election's encryption key. This exporter accepts the event's default key ceremony.
It checks the key ceremony, board reference, encryption policy and expected batch set.
This keeps each scope consistent with the saved tally records.
The auditor compares `tally-plan.json` with the expected election coverage because the plan derives from the supplied rows.

The exporter reads application, eligibility and B3 records directly from local database copies.
The electoral-log exporter reads the local ImmuDB copy through its native interface.
The resulting files retain the source records and their digests for the subsequent checks.

## 2. Reproduce ballot selection

Preparation reproduces the platform's selection rules for each scope.
A **cast row** is a ballot record in the application database.
For each voter, the tool selects the most recent row marked valid in that database.
It orders rows by stored timestamp, with the row ID as a tie-breaker.

If no valid row exists, it retains the latest discarded row for exclusion accounting.
A row with status `in-progress` in the scope makes preparation incomplete.

The tool applies the supplied eligibility records and records each selection decision in `selection.json`.
Reasons include `selected`, `superseded_revote`, `discarded` and `no_eligible_voter`.
This lets the auditor reconcile stored cast records with the selected inputs and explain exclusions.
The result reproduces the recorded decisions; the authority supplies evidence that the eligibility and administrative decisions were authorized.

**Multiplicity** is the ballot's vote value under the selected policy: one, the voter's weight, or one plus counted delegations.
The exporter handles weighted and delegated voting separately; it rejects configurations that combine them.

| Policy | How the encrypted inputs are reconstructed |
| --- | --- |
| Unweighted | One ciphertext in the scope's batch for each selected ballot. |
| Delegated | One copy plus the counted delegations, in the scope's batch. |
| Weighted | One copy in each batch for the powers of two that sum to the voter's weight. |

A **batch offset** counts positions from the scope's base batch number (`base_batch` in `tally-plan.json`).
The scope's batches need not include the base batch itself.
For weight 5, the tool places one copy at offset 0 and one at offset 2.
The later count applies multipliers of 1 and 4, giving a vote value of 5.
In general, the multiplier for offset `b` is `2^b`.
This stage verifies the input placement that supports that calculation.

Area-weighted voting uses unweighted input reconstruction here; area weights belong to the later counting stage.
Delegation reconstruction follows the platform's rule, including its treatment of disabled and cross-realm delegators, with diagnostics for those cases.
A **realm** is a separate set of users and settings in Keycloak.

Where available, preparation compares its voter and ballot counts with saved tally summaries, called **annotations**.
These checks cover eligible voters, cast ballots, exclusions, and applicable channel counts and weight-batch information.
`provenance.json` identifies the comparisons made and any older fields that were unavailable.

## 3. Verify the receipts and signed cast records

For every selected ballot, the tool recomputes the ballot hash using the platform's own format.
**Borsh** turns structured data into an exact sequence of bytes. The hash covers the version, issue date and encrypted contest data.
The tool first decodes the stored data format to read the ciphertexts; the vote choices remain encrypted.

The receipt is the first 32 bytes of the SHA-512 digest, written as 64 hexadecimal characters.
The signed electoral-log statement contains the full 64-byte ballot hash.
The audit checks both the receipt and the full signed hash.
A separate digest checks the exact stored content bytes against the prepared manifest.
Fields outside the native ballot hash rely on the authenticated source evidence; the run's new digests detect subsequent changes.

The tool verifies the electoral-log system signature against the pinned system key.
It also checks the sender signature against the sender key carried in the message.
The independently authenticated signing anchor is the system key.
These signatures authenticate the logging system's recorded cast statement.
It compares the signed election event, election, hashed voter identifier and full ballot hash with the selected stored ballot.

The **hashed voter identifier** is the SHA-512 hash of the Borsh-serialized voter ID from the stored cast row.
The code and its error messages call this value a voter pseudonym (`PseudonymHash`).
The verifier recomputes it from `voter_id_string`; both supported signed cast-record formats contain this hash.
It lets the auditor match records without relying on unsigned outer message labels.
For newer messages, the tool also compares the signed voting channel.
For legacy messages without a channel, it infers `ONLINE` and reports that inference.

Preparation requires one distinct signed message for each selected ballot and collapses byte-identical repetitions.
It rejects several selected cast votes that share a receipt.
These checks establish agreement between the selected encrypted ballots and the authenticated cast statements.

## 4. Reconcile cast history and revotes

Preparation normally checks the complete supplied cast history for the election, across all areas.
It confirms that each selected row occurs under the same hashed voter identifier and full ballot hash.
Every signed ballot of a selected voter must also have a stored row.
For each selected voter, preparation looks for a ballot signed later than the selected one.
If another row marked valid in the same scope holds that later ballot, preparation fails.
If a ballot was signed more than once, it uses its latest signing time.

This adds signed timing evidence to the database's revote order.
The timestamps use whole seconds from the clock of the server that signed the record.
Their interpretation depends on synchronized clocks and cannot distinguish casts within one second.
`audit-tally` always includes these history checks.

`audit-cast-history` separately reconciles all supplied rows and signed cast records for each election represented in the tally.
It includes superseded and discarded ballots across all areas.

| Comparison | Reported result |
| --- | --- |
| Matching hashed voter identifiers and full ballot hashes in both sources | Consistent coverage of the supplied records. |
| Signed cast record without a stored ballot | Contradiction, assuming the Hasura backup includes every cast recorded in the ImmuDB backup. |
| Stored ballot without a signed cast record | Incomplete evidence; the log write can fail after acceptance. |
| Several rows or distinct messages for the same pair of hashes | Ambiguous attribution; the history result is incomplete. |
| Byte-identical repeated signed messages | Reported repetitions, rather than separate proven casts. |

This verifies signed-message coverage within the supplied records.
Database inclusion and history proofs are a separate form of evidence.

## 5. Compare the encrypted tally inputs

`ballot-audit` checks the prepared bundle against its pins.
Within `audit-tally`, those pins connect the export, preparation and audit steps in the same trusted run.
For the selected B3 `Ballots` messages, it verifies signatures, configuration references, artifact hashes and the expected public key hash.

It extracts the relevant ciphertext from every selected ballot and compares the complete expected input with the selected B3 batches.
The comparison uses a **multiset**: a collection that preserves how many times each value occurs.
Ciphertext values, batch numbers and occurrence counts must all agree.
This detects omitted, additional, substituted or incorrectly duplicated inputs, including weighted and delegated contributions.
An empty selection still needs its authenticated empty input batch.

A receipt lookup displays matching encrypted input positions after the entire selected bundle is checked.
Different receipts can carry identical ciphertexts; the report lists every matching position without assigning a unique origin to each copy.
Exclusion explanations use stored selection labels, which the lookup identifies as unverified labels.

The input audit and Braid each read the signed `Ballots` message for every batch from the same restored board.
The input audit checks its artifact hash and accepts one such message per scoped batch.
Braid checks that the first mix starts from the `Ballots` hash it reads.
No digest passes between the tools, so this connection relies on the authenticated B3 copy.

## 6. Check the cryptographic tally

A **mix** shuffles and re-encrypts ciphertexts, changing their order and encrypted representation while preserving their concealed values.
A **shuffle proof** lets the verifier check this relationship without learning the secret new order.
The **protocol manager** submits the encrypted ballot batches to B3.
**Trustees** are participants in the cryptographic protocol. The configuration sets their keys and the required participation **threshold**.

Braid uses the public board records to verify:

- The configuration, trustee set and threshold, message signatures, and common configuration references.
- The public key and trustee verification keys reconstructed from public data, including the required trustee signatures.
- The protocol manager's binding between the encrypted ballot batch and the public key.
- The shuffle proofs, required chain length, distinct trustees, and connection from input ballots to the final mix.
- The published plaintext records' references to the public key and final mix.

Trustees publish **decryption factors**: partial decryptions used together to recover the plaintexts.
Braid checks the decryption proofs, recomputes the plaintexts from these factors, and compares them with the published plaintexts.
A batch is verified only when the verifier has signed its own recomputation; a published plaintext record alone is not enough.

The `DECRYPTION_VALID` and `PLAINTEXTS_VALID` report fields establish reference consistency within plaintext records.
The recomputation is a separate condition of the success.
Revisions before `586cfe5e1b7` do not require it; do not use them to report verified decryption.
The auditor uses public evidence rather than election private keys.

`audit-tally` passes Braid the tally's batches together with every `Ballots` batch found on the board.
Braid checks this exact set, with at least one batch and plaintext evidence for every required batch.
A batch from another tally on the same board makes Braid, and the audit, fail if its required plaintext record is missing.
Braid returns a nonzero exit status if any required report check fails or has no result, including checks within each batch.

## What a successful run establishes

With authenticated inputs and the supported tool version, a successful run connects signed cast records to the selected encrypted tally inputs and verified mixes.
The auditor can identify the checked scopes, reconcile the selection, and inspect the signed plaintext records associated with the final mix.
Braid has recomputed the plaintexts of every batch and found that they match the published plaintexts.
These are input and cryptographic checks; assessment of official candidate totals also requires the separate final counting and results-reconciliation steps.

The overall status combines the individual steps:

| Exit status | Interpretation |
| --- | --- |
| `0` | All recorded steps reported success. Read their evidence and qualifications before reporting a pass. |
| `1` | A step failed verification or crashed, Braid failed, or image setup failed. Investigate the recorded cause. |
| `3` | Evidence was incomplete, attribution was ambiguous, or a required step did not run. |
| `4` | The input or setup was invalid or unsupported. Command-line usage errors can instead return `2`. |

A successful scope alone does not establish overall success.
The auditor also checks restore logs, expected scope coverage and required output files.
If the final `ballot-audit: exit status` line is absent, read the setup error before interpreting the exit number.
See [Read the Results](03-ballot_audit_manual/03-read-the-results.md) for status precedence, restore warnings and unavailable annotation checks.

## Audit records and privacy

`tally-plan.json` records the scope. `audit-record.json` records commands, statuses and output digests.
The per-scope reports preserve the selection and matching evidence; the history reports explain coverage.
`public-summary.json` provides tally, tenant, election, area and contest identifiers, batch information and aggregate counts without voter IDs or receipts.
The [File Reference](03-ballot_audit_manual/06-file-reference.md) explains each file.

Keep the detailed evidence private: it contains voter references, receipts and encrypted ballot links.
Signed cast records also contain IP addresses and country information.
The hash uses no additional random value; anyone who knows the voter ID can reproduce it.
The database and unsigned log metadata can also contain the voter ID. The unsigned metadata can also contain the username.

Review aggregate summaries before publication, especially for small groups.
The tool flags weighted batches that contain one to four ciphertexts.
Small batches can make individual decrypted votes easier to associate with voters.
Review these warnings before publication even when verification checks pass.

The documented image command uses `--network none` and a read-only backup mount.
Local database and B3 services run inside the container; exporters use read-only accounts and transactions on the restored copies.
The manual explains workstation preparation, compatible versions and handling of the copies.

## Implementation and evidence

This explanation describes proposed `release/10.0` code in
[PR #3968](https://github.com/sequentech/step/pull/3968), at commit
[`dc593d2e2f9`](https://github.com/sequentech/step/commit/dc593d2e2f9c9b6eae7a13ca8b07025c6982ce1e).
For release status, image validation and version selection, see
[Versions](03-ballot_audit_manual/01-before-you-start.md#versions).

The technical sources are the
[ballot audit implementation and tests](https://github.com/sequentech/step/tree/dc593d2e2f9c9b6eae7a13ca8b07025c6982ce1e/packages/ballot-audit),
[Braid verification checks](https://github.com/sequentech/step/blob/dc593d2e2f9c9b6eae7a13ca8b07025c6982ce1e/packages/braid/src/verify/verifier.rs),
[success rules](https://github.com/sequentech/step/blob/dc593d2e2f9c9b6eae7a13ca8b07025c6982ce1e/packages/braid/src/verify/datalog.rs),
and [native receipt hashes](https://github.com/sequentech/step/blob/dc593d2e2f9c9b6eae7a13ca8b07025c6982ce1e/packages/sequent-core/src/encrypt.rs).
The [parent issue](https://github.com/sequentech/meta/issues/14374) records the acceptance work and synthetic test evidence.

To run the process, use the [Ballot Audit Manual](03-ballot_audit_manual/00-ballot_audit_manual.md).
