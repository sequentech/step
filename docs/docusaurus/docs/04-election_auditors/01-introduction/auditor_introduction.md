---
id: auditor_introduction
title: Introduction
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This part is for persons who check an election after it ends: auditors, observers and the
representatives of candidates. It tells you which evidence the platform gives, what each piece of
evidence shows and what it does not show.

## What you can check

| Property | Evidence | Page |
| --- | --- | --- |
| The ballot box was empty at the start. | The initialization report. | [Election results verification](../02-tutorials/01-auditor_results-verification.md) |
| The results agree with the votes cast. | The results document, the participation report and the ballot images. | [Election results verification](../02-tutorials/01-auditor_results-verification.md), [Ballot image audit](../02-tutorials/02-auditor_ballot-image-audit.md) |
| The important actions occurred at the correct times, by the correct persons. | The electoral log. | [Immutable election logs](../02-tutorials/03-auditor_election-logs.md) |
| A ballot contains the choices of the voter. | The ballot audit of the voter, with the ballot verifier. | [Election results verification](../02-tutorials/01-auditor_results-verification.md#what-a-voter-can-check) |
| The software does what the documentation says. | The public source code. | [Source code review](../02-tutorials/05-auditor_source-code-review.md) |

## How the platform keeps the vote secret

1. The device of the voter encrypts the ballot before it sends it. The platform stores only the
   encrypted ballot.
2. In the key ceremony, the trustees make the election key together. Each trustee keeps one
   fragment. Decryption needs a minimum number of trustees (the threshold). One person alone,
   including Sequent, cannot decrypt the votes.
3. In the tally ceremony, the platform shuffles the encrypted ballots (a mixnet) before it
   decrypts them. After the shuffle, no ballot can be linked to a voter.
4. The trustees upload their fragments and the platform decrypts and counts the shuffled
   ballots.

For the details, see the [Threat model](../../06-technology/01-threat-model.md), the
[White paper](../../06-technology/02-white-paper.md) and the
[Cryptographic protocol specification](../../06-technology/03-cryptographic-protocol-spec.md).

## How to get the evidence

The election administrator collects the evidence after the election. Some evidence contains voter
data and stays inside the election organization. See
[Prepare the Audit Pack](../../02-election_managers/03-procedures/08-audit-pack.md) for the list of documents and which
ones you can receive.
