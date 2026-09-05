---
id: acclaimed_contests
title: Acclaimed Contests
sidebar_position: 11
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

An **acclaimed contest** is decided before voting: its configured candidates
are elected without a vote. The contest remains visible throughout the voter
and results experience, but it never becomes part of an encoded ballot.

The feature follows one rule:

> **An acclaimed contest is displayed but never encoded.**

This is a per-contest setting. An election may contain both acclaimed and
normally voted contests.

## Configuration

1. Open the target Contest in the Admin Portal.
2. Go to the **Data** tab and expand **Ballot Design**.
3. Enable **Decided by acclamation**.
4. Confirm that the configured candidates are the people who should be elected.
5. Save the Contest and generate a new ballot publication.

Configure acclamation before publishing ballots. The setting determines which
contests participate in ballot encoding, so changing it after publication can
invalidate existing Ballot IDs and make ballots incompatible with their
published definition.

See [Contest Data](./04-contest/01-election_management_contest_data.md) for the
other contest settings.

### Acclamation notice

By default, the Voting Portal and Ballot Verifier use the standard translated
acclamation notice. A contest can override it per language in its presentation
configuration:

```json
{
  "i18n": {
    "en": {
      "acclamation_description": "This contest was decided by acclamation."
    },
    "fr": {
      "acclamation_description": "Ce vote a été acquis par acclamation."
    }
  }
}
```

The current language is used when available, followed by the configured
default language and then the application's standard translated notice.

## Voting Portal behavior

The ballot and review screens show the acclaimed contest in the normal contest
order, including its notice and available candidates. Candidate options are
disabled and cannot be selected.

The following settings do not apply to an acclaimed contest:

- minimum and maximum selection requirements;
- under-vote, over-vote, blank-vote, and invalid-vote policies;
- explicit blank and invalid options; and
- write-in entry.

### Mixed election

When an election contains both acclaimed and normally voted contests, the voter
completes only the normally voted contests. Those selections produce the usual
encrypted ballot, Ballot ID, audit option, receipt, and tracking flow. The
acclaimed contest contributes nothing to that ballot.

### Fully acclaimed election

When every contest on the voter's ballot is acclaimed, there is nothing to
cast. The voter can review what was decided, but:

- no ballot is encrypted or submitted;
- no Ballot ID, QR code, tracking link, receipt, or audit option is created; and
- the confirmation screen explicitly states that no ballot was cast.

## Ballot Verifier behavior

An acclaimed contest has no decoded selection because it is not encoded. For a
mixed election, the Ballot Verifier combines the decoded selections with the
acclaimed contests from the embedded ballot configuration for display. It then
shows the same acclamation notice and disabled candidates as the Voting Portal
review screen.

This merge is presentation-only. It does not add the contest to decoded ballot
data and does not change ballot hashing, signature checks, re-encoding checks,
or Ballot ID comparison. A fully acclaimed election has no auditable ballot to
open in the verifier.

See [Audit your Vote](../../03-voters/01-tutorials/03-voter_audit_ballot.md) for
the complete voter audit procedure and
[Ballot Encoding Specification](../../05-reference/07-ballot_encoding.md) for
the encoding rules.

## Tally and results

Because no ballots contain the acclaimed contest, the tally creates its result
from the published contest configuration rather than running the configured
counting algorithm. The result contains:

- zero census, participation, votes, blank votes, invalid votes, candidate
  totals, and percentages;
- every eligible configured candidate marked as a winner, in configured order,
  with winning positions `1..N`; and
- no counting process or participation-by-channel data.

Explicit blank and invalid markers, disabled candidates, and empty write-in
slots are not treated as elected candidates. The Admin Portal, public Results
Portal and PDF report show an acclamation notice instead of normal
participation figures.

Tally sheets are rejected for acclaimed contests because paper, postal, and
other external votes cannot be added to a contest that was decided without a
vote.

See [Generating Results](../../05-reference/04-tally_deep_dive/06-tally_results.md)
for the canonical tally result shape.

## Summary

| Aspect | Behavior |
|--------|----------|
| **Configuration** | Contest **Data** → **Ballot Design** → **Decided by acclamation** |
| **When to configure** | Before ballot publication; do not change it after publishing |
| **Voter interaction** | Contest and candidates shown, but every option is disabled |
| **Ballot encoding** | No plaintext position, ciphertext, or decoded contest |
| **Mixed election** | Other contests are cast and audited normally |
| **Fully acclaimed election** | No ballot, Ballot ID, receipt, tracker, or audit |
| **Verifier** | Adds the contest from ballot configuration for display only |
| **Tally** | Synthetic zero result; every eligible candidate is a winner |
| **Participation** | Suppressed and replaced by an acclamation notice |
| **Tally sheets** | Rejected |
