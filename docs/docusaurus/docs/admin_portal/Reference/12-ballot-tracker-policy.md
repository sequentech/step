---
id: ballot_tracker_policy
title: Ballot Tracker Policy
sidebar_position: 12
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **ballot tracker** (the Ballot ID shown to the voter) is a hash of the
encrypted ballot. From ballot version 3 on it also covers the ballot format
(single-contest or multi-contest) and the ballot style the ballot was encrypted
with. The ballot style covers the election, the area, the election public key
and the contests and candidates in their order, so a ballot only matches its
tracker under that exact ballot style.

When a vote is cast, the server checks that a version 3 ballot names a ballot
style currently published for the voter's area and election, and that its
ballot style hash matches that ballot style.

Ballots of version 2 have a tracker that covers only the issue date and the
contests. Ballots cast with version 2 keep their tracker and still verify. The
**ballot tracker policy** decides whether new version 2 ballots are accepted.

## Values

The policy is stored in the election event presentation, in the
`ballot_tracker_policy` field.

| Value | Wire value | New version 2 ballots |
|---|---|---|
| **Style bound** (default) | `style-bound` | Rejected |
| **Allow legacy** | `allow-legacy` | Accepted without the ballot style check |

When the field is absent the policy is **Style bound**. Version 3 ballots are
checked against the published ballot style under both values.

Use **Allow legacy** only while a voting client that still produces version 2
ballots is in use.

## Rejected ballots

A ballot that fails these checks is rejected and the voting portal shows the
`CAST_VOTE_DeserializeBallotFailed` message. The most common cause is a voting
portal that was loaded before a new ballot publication: reloading the voting
portal loads the current ballot style.

The voting portal and the server both compute the ballot style hash, so they
must run the same release. Upgrade them together.
