---
id: slates_configuration
title: Slate Configuration
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Slate Configuration

A slate is a named group of candidates that run together across the contests of one election. Configuring slates tells the platform which candidates each slate has in which contest. A slate is never a vote of its own: every choice stays an ordinary candidate vote in its contest, counted as it always is.

An election with no slate configuration is unaffected. Its ballots, their hashes and the voting flow stay exactly as they are.

## Where it is configured

Open the election, go to the **Data** tab and expand **Advanced Configuration**:

- **Slate configuration (JSON)** holds the slates. Clear the field to remove slates from the election.
- **Mobile candidate lists** chooses whether each slate's candidate list starts **Collapsed** or **Expanded** on phones. It edits the same value in the JSON, so the two always agree. It is available once the field holds a readable configuration.

Saving checks the configuration against the election's contests and candidates and lists every problem found. Nothing is stored until the configuration is valid.

The configuration is stored in the election's `sequent.slates` annotation. A change reaches voters only through a new ballot publication.

## Format

```json
{
  "version": 1,
  "mobile_candidate_lists": "collapsed",
  "slates": [
    {
      "id": "forward-together",
      "name": {"en": "Forward Together", "es": "Adelante Juntos"},
      "members": {
        "<president contest id>": ["<candidate id>"],
        "<trustees contest id>": ["<candidate id>", "<candidate id>", "<candidate id>"]
      }
    },
    {
      "id": "independent-voices",
      "name": {"en": "Independent Voices"},
      "members": {
        "<trustees contest id>": ["<candidate id>"]
      }
    }
  ]
}
```

| Field | Meaning |
| --- | --- |
| `version` | Always `1`. |
| `mobile_candidate_lists` | `collapsed` or `expanded`. Optional; `collapsed` when absent. |
| `slates` | The slates, in the order they are shown. |
| `slates[].id` | A stable identifier: up to 64 letters, digits, hyphens or underscores, starting with a letter or digit. Unique within the election. |
| `slates[].name` | The name shown to voters, by language code. A name in the election's default language is required; other languages fall back to it. Plain text on one line, up to 120 characters, unique per language. |
| `slates[].members` | The slate's candidates, by contest. Keys are contest ids of this election and values are candidate ids of that contest. |

Contest and candidate ids are the ones shown in the Admin Portal for each contest and candidate. Names are never used as references, so renaming a candidate does not change a slate.

A slate does not need to cover every contest, and it may list fewer candidates than a contest allows. A slate with candidates for trustees only is a valid, partial slate. Candidates that belong to no slate remain on the ballot as independent candidates.

## Rules

Contest limits are the contests' own **Minimum votes** and **Maximum votes**; slates add no limits of their own. The configuration is rejected when:

- it is not valid JSON, has a different `version`, has fields other than the ones above, or is larger than 64 KiB
- a slate id is repeated or malformed
- a slate has no name, an empty name, no name in the election's default language, or a name already used by another slate in the same language
- a slate has no members, or lists a contest without candidates
- a contest is not in the election, or a candidate is not in the contest it is listed under
- a candidate is listed twice or belongs to more than one slate
- a slate lists more candidates in a contest than the contest's maximum votes
- a contest uses a preferential or cumulative counting algorithm; slates support ordinary candidate voting only
- a member is disabled, a write-in placeholder, an explicit blank or invalid option, or a category list

## When it is checked

The same rules are applied at every step, so a configuration cannot pass one and fail the next:

| Step | What happens on a problem |
| --- | --- |
| Saving the election in the Admin Portal | The field shows every problem and the election is not saved. |
| Importing an election event | The import is refused and reports the problems. Contest and candidate ids inside the configuration are regenerated together with the rest of the event, so imported slates keep pointing at the imported candidates. |
| Generating a ballot publication | The generation task fails with the problems. The configuration is checked against the whole election first and then against each area's ballot. |
| Publishing a generated publication | Publishing is rejected with the problems. This also covers publications generated before an upgrade. |
| Loading the ballot in the Voting Portal | The voter sees a configuration error instead of a ballot that silently lacks its slates. |

Each voter's ballot only contains the contests their area votes on. Contests outside it are left out of the slate for that voter. A member missing from a contest that is on the ballot is an error.

## After voting starts

Once voting has started for an election, a publication that changes its slate configuration is rejected: voters who have not voted yet must see the same slates as those who already have. Restore the published configuration to publish other changes. Reformatting the JSON is not a change.
