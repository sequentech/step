---
id: slates_configuration
title: Slate Configuration
sidebar_position: 14
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This page explains where an Election's slates are configured, the rules the Election imposes on them and when those rules are checked. The format of the configuration and what voters see are described in [Named Slates](./13-named-slates.md).

An Election with no slate configuration is unaffected. Its ballots, their hashes and the voting flow stay exactly as they are.

---

## Where it is configured

Open the Election, go to the **Data** tab and expand **Advanced Configuration** (see [Election Data](./03-election/03-election_management_election_data.md)):

- **Slate configuration (JSON)** holds the slates. Clear the field to remove slates from the Election.
- **Mobile candidate lists** chooses whether each slate's candidate list starts **Collapsed** or **Expanded** on phones. It edits `mobile_candidate_lists` in the JSON, so the two always agree. It is available once the field holds a readable configuration.

Saving checks the configuration against the Election's contests and candidates and lists every problem found. Nothing is stored until the configuration is valid. Other annotations of the Election are kept as they are.

Contest and candidate identifiers are the ones the Admin Portal shows for each contest and candidate. Names are never used as references, so renaming a candidate does not change a slate.

---

## Rules the Election imposes

Contest limits are the contests' own **Minimum votes** and **Maximum votes**; slates add no limit of their own. On top of the format rules in [Named Slates](./13-named-slates.md#validation), the configuration is refused when:

- a slate has no name in the Election's default language;
- a slate lists more candidates in a contest than the contest's maximum votes;
- a contest uses a preferential or cumulative counting algorithm, since slates support ordinary candidate voting only;
- a member is disabled, a write-in placeholder, an explicit blank or invalid option, or a category list.

A slate that covers only some contests, or lists fewer candidates than a contest allows, is a partial slate and is valid.

---

## Listing order

Election rules often require a fixed listing: full slates first, then partial slates, then independent candidates, with the order among full slates and among partial slates drawn by lot. The configuration expresses it in two places:

- **The order of the `slates` array** is the order of the slate cards: left to right on wide screens, top to bottom on phones. List the full slates first, in the drawn order, then the partial slates.
- **Each contest's candidate order** is the order of the individual candidates. In the contest's **Ballot Design**, set the candidate order to **Custom** and use **Reorder candidates** to list the candidates of the first slate first, then those of the next slate, and the independent candidates last.

The platform does not impose a listing order: the configured order is the order voters see.


---

## When it is checked

The same rules apply at every step, so a configuration cannot pass one and fail the next:

| Step | What happens on a problem |
| --- | --- |
| Saving the Election in the Admin Portal | The field shows every problem and the Election is not saved. |
| Importing an Election Event | The import is refused and reports the problems. Contest and candidate identifiers inside the configuration are regenerated together with the rest of the Election Event, so imported slates keep pointing at the imported candidates. |
| Generating a ballot publication | The generation task fails with the problems. The configuration is checked against the whole Election first and then against each area's ballot. |
| Publishing a generated publication | Publishing is rejected with the problems. This also covers publications generated before an upgrade. |
| Loading the ballot in the Voting Portal | The voter sees a configuration error instead of a ballot that lacks its slates. |

Each voter's ballot only contains the contests their area votes on. A slate's contests outside that ballot are not checked for that voter. A member missing from a contest that is on the ballot is an error.

---

## After voting starts

Once voting has started for an Election, a publication that changes its slate configuration is rejected: voters who have not voted yet must see the same slates as those who already have. Restore the published configuration to publish other changes. Reformatting the JSON is not a change.
