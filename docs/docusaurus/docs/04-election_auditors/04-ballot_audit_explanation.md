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
| Signed cast records | The recorded voter pseudonym, election and encrypted ballot agree with the signed electoral log. |
| Ballot selection | The selected ballots follow the supported revote, eligibility, weight and delegation rules. |
| Encrypted tally inputs | The correct batches contain exactly the selected ciphertexts, with the expected number of copies. |
| Cryptographic tally | The public evidence supports the key construction and shuffle, with plaintext records bound to the same tally. |

A **ciphertext** is an encrypted value that conceals the vote choices. A **plaintext** is the decrypted value.
A **batch** is a numbered group of ciphertexts on the B3 bulletin board.
A **tally session** identifies one tally, which can cover several elections, areas and contests.

## Evidence and trusted starting values

The process uses restored copies of four sources:

| Source | Evidence used |
| --- | --- |
| Application database, also called Hasura | Cast ballots, receipts, voter references, row status, timestamps, tally configuration and saved area/contest mappings. |
| Keycloak database | Voter eligibility, area and election permissions, vote weights and delegations. |
| B3 database and any separate blob files | Signed protocol messages, configuration, public key information, encrypted input batches, mixes and decryption evidence. |
| ImmuDB database | The exact bytes of signed `CastVote` messages in the electoral log. |

The election authority supplies the tally session ID, public key artifact digest and electoral-log system public key through an independently trusted channel.
An **artifact** is a data object attached to a protocol message, such as public key information or an encrypted ballot batch.
The board configuration supplies the protocol-manager and trustee keys.
The supplied public-key digest and system key anchor the checks to the authority's expected election.

The auditor authenticates the backup digests, capture records and tally scope before the run.
The backups must preserve the intended tally-time records, including **frozen eligibility**: the eligibility, weight and delegation state used for selection.
The exporter reads that state from the supplied Keycloak copy.
Stable read-only transactions preserve each export's view.
The auditor uses capture records to assess consistency between the separate source snapshots.

A **digest** is a fixed-length value computed from data. SHA-512 produces a 64-byte digest.
A **pin** is an expected digest used to detect a change in those bytes.
Digests computed during the run connect one step's output to the next step's input.
They establish consistency within the run; historical authenticity depends on the independently trusted sources and capture evidence.

## 1. Confirm the tally scope

`audit-tally` creates one **scope** for each saved `tally_session_contest` row.
Each scope identifies the tenant, election event, election, area, contest, tally session, board and batch numbers.
Some supported ballot formats combine several contests in one encrypted value.

The exporter compares these identifiers with the saved tally configuration and mappings.
It checks the key ceremony, board reference, encryption policy and expected batch set.
This keeps each scope consistent with the saved tally records.
The auditor compares `tally-plan.json` with the expected election coverage because the plan derives from the supplied rows.

The exporter reads application, eligibility and B3 records directly from local database copies.
The electoral-log exporter reads the local ImmuDB copy through its native interface.
The resulting files retain the source records and their digests for the subsequent checks.

## 2. Reproduce ballot selection

Preparation reproduces the platform's selection rules for each scope.
For each voter, it selects the latest valid row by stored timestamp, with the row ID as a tie-breaker.
If no valid row exists, it retains the latest discarded row for exclusion accounting.
An in-progress row makes preparation incomplete.

The tool applies the supplied eligibility records and records each selection decision in `selection.json`.
Reasons include `selected`, `superseded_revote`, `discarded` and `no_eligible_voter`.
This lets the auditor reconcile stored cast records with the selected inputs and explain exclusions.
The result reproduces the recorded decisions; the authority supplies evidence that the eligibility and administrative decisions were authorized.

**Multiplicity** is the ballot's vote value under the supported policy: one, the voter's weight, or one plus counted delegations.

| Policy | How the encrypted inputs are reconstructed |
| --- | --- |
| Unweighted | One ciphertext in the scope's batch for each selected ballot. |
| Delegated | One copy plus the counted delegations, in the scope's batch. |
| Weighted | One ciphertext per set bit of the voter's weight, at its offset from the base batch. Weight 5 uses offsets 0 and 2. |

For weighted voting, the later count applies the factor `2^b` to batch offset `b`.
This stage verifies the input placement that supports that calculation.
Area-weighted voting uses unweighted input reconstruction here; area weights belong to the later counting stage.
Delegation reconstruction follows the platform's rule, including its treatment of disabled and cross-realm delegators, with diagnostics for those cases.

Preparation also compares reconstructed accounting with available saved tally annotations.
These checks cover eligible voters, cast ballots, exclusions, and applicable channel counts and weight-batch information.
`provenance.json` identifies the comparisons made and any older fields that were unavailable.

## 3. Verify the receipts and signed cast records

For every selected ballot, the tool recomputes the platform's native ballot hash.
It uses **Borsh**, a defined binary serialization format, over `RawHashableBallot` or its multi-contest equivalent.
The hash covers the version, issue date and decoded encrypted contest data.

The receipt is the first 32 bytes of the SHA-512 digest, written as 64 hexadecimal characters.
The signed electoral-log statement contains the full 64-byte ballot hash.
The audit checks both the receipt and the full signed hash.
A separate digest checks the exact stored content bytes against the prepared manifest.
Fields outside the native ballot hash rely on the authenticated source evidence; the run's new digests detect subsequent changes.

The tool verifies the electoral-log system signature against the pinned system key.
It also checks the sender signature against the sender key carried in the message.
The independently authenticated signing anchor is the system key.
It compares the signed election event, election, voter pseudonym and full ballot hash with the selected stored ballot.

A **pseudonym** here is a deterministic hash of the serialized voter ID.
It lets the auditor match records without relying on unsigned outer message labels.
For newer messages, the tool also compares the signed voting channel.
For legacy messages without a channel, it infers `ONLINE` and reports that inference.

Preparation requires one distinct signed message for each selected ballot and collapses byte-identical repetitions.
It rejects several selected cast votes that share a receipt.
These checks establish agreement between the selected encrypted ballots and the authenticated cast statements.

## 4. Reconcile cast history and revotes

Preparation normally checks the complete supplied cast history for the election, across all areas.
It confirms that each selected row occurs under the same voter pseudonym and full ballot hash.
Every signed ballot of a selected voter must also have a stored row.
The tool checks whether another valid scoped row has a later signed ballot for that voter, using each ballot's latest signing.

This adds signed timing evidence to the database's revote order.
The timestamps use whole seconds from the worker's clock, so their interpretation depends on synchronized clocks and cannot distinguish casts within one second.
`audit-tally` always includes these history checks.

`audit-cast-history` separately reconciles all supplied rows and signed cast records for each election represented in the tally.
It includes superseded and discarded ballots across all areas.

| Comparison | Reported result |
| --- | --- |
| Matching voter/hash pairs in both sources | Consistent coverage of the supplied records. |
| Signed cast record without a stored ballot | Contradiction, assuming the Hasura backup includes every cast recorded in the ImmuDB backup. |
| Stored ballot without a signed cast record | Incomplete evidence; the log write can fail after acceptance. |
| Several rows or distinct messages for one voter/hash pair | Ambiguous attribution; the history result is incomplete. |
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

The input audit and Braid use the same signed `Ballots` message for each batch.
The input audit checks its artifact hash; Braid checks that the first mix starts from that hash.
This connects the reconstructed selection to the cryptographic verification that follows.

## 6. Check the cryptographic tally

A **mix** shuffles and re-encrypts ciphertexts, changing their order and encrypted representation while preserving their concealed values.
A **shuffle proof** lets the verifier check this relationship without learning the secret permutation.
**Trustees** are participants in the cryptographic protocol. The configuration sets their keys and the required participation **threshold**.

Braid uses the public board records to verify:

- The configuration, trustee set and threshold, message signatures, and common configuration references.
- The public key and trustee verification keys reconstructed from public data, including the required trustee signatures.
- The protocol manager's binding between the encrypted ballot batch and the public key.
- The shuffle proofs, required chain length, distinct trustees, and connection from input ballots to the final mix.
- The published plaintext records' references to the public key and final mix.

When its decryption verification routine runs, Braid checks decryption proofs, recomputes plaintexts from public factors, and compares them with published plaintexts.
At the reviewed revision, the success condition does not require this routine to have completed for every batch.

The `DECRYPTION_VALID` and `PLAINTEXTS_VALID` report fields establish reference consistency within plaintext records.
They do not establish that recomputation ran.
The current success condition establishes the other checks listed above.
Before reporting verified decryption, use a verifier revision that requires successful recomputation for every batch.
The auditor uses public evidence rather than election private keys.

`audit-tally` passes Braid the tally's batches together with every `Ballots` batch found on the board.
Braid checks this exact set, with at least one batch and plaintext evidence for every required batch.
An unfinished batch from another tally on the same board therefore makes Braid, and the audit, fail.
At the reviewed revision, failed or unpopulated required checks, including nested batch checks, produce a nonzero exit status.

## What a successful run establishes

With authenticated inputs and the supported tool version, a successful run connects signed cast records to the selected encrypted tally inputs and verified mixes.
The auditor can identify the checked scopes, reconcile the selection, and inspect the signed plaintext records associated with the final mix.
Reporting verified decryption requires a verifier revision that enforces successful recomputation for every batch.
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
`public-summary.json` provides election identifiers, batch information and aggregate counts without voter IDs or receipts.
The [File Reference](03-ballot_audit_manual/06-file-reference.md) explains each file.

Keep the detailed evidence private: it contains voter references, receipts and encrypted ballot links.
Signed cast records also contain IP addresses and country information.
The pseudonym is an unsalted hash that anyone with the voter ID can reproduce.
Review aggregate summaries before publication, especially for small groups and the reported small weighted-batch privacy warnings.

The documented image command uses `--network none` and a read-only backup mount.
Local database and B3 services run inside the container; exporters use read-only accounts and transactions on the restored copies.
The manual explains workstation preparation, compatible versions and handling of the copies.

## Implementation and evidence

This explanation describes proposed `release/10.0` code in
[PR #3968](https://github.com/sequentech/step/pull/3968), at commit
[`820352256b8d`](https://github.com/sequentech/step/commit/820352256b8da1fe5b6c6e75e6b4928d58c1c319).
For release status, image validation and version selection, see
[Versions](03-ballot_audit_manual/01-before-you-start.md#versions).

The technical sources are the
[ballot audit implementation and tests](https://github.com/sequentech/step/tree/820352256b8da1fe5b6c6e75e6b4928d58c1c319/packages/ballot-audit),
[Braid verification checks](https://github.com/sequentech/step/blob/820352256b8da1fe5b6c6e75e6b4928d58c1c319/packages/braid/src/verify/verifier.rs),
and [native receipt hashes](https://github.com/sequentech/step/blob/820352256b8da1fe5b6c6e75e6b4928d58c1c319/packages/sequent-core/src/encrypt.rs).
The [parent issue](https://github.com/sequentech/meta/issues/14374) records the acceptance work and synthetic test evidence.

To run the process, use the [Ballot Audit Manual](03-ballot_audit_manual/00-ballot_audit_manual.md).
