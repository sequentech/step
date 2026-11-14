---
id: import_format_election_contest_candidate
title: Elections, Contests and Candidates in the Election Event File
sidebar_position: 2
description: "The elections, contests and candidates arrays of the election event JSON file: every field, the allowed values, how the objects refer to each other, and an example."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This page gives the format of the `elections`, `contests` and `candidates` arrays of the election
event JSON file. The other parts of the file are on [Election Event File](01-election-event.md).
Use this page to read an export or to check a file before you import it.

:::note NOTE: election event file
Make the election event file with **Export** on the election event. Change the file by hand only
if you must, and only the fields on this page. Import the changed file into a test election event
first.
:::

## How the objects are linked

The file has three flat arrays. The objects refer to each other through IDs:

```text
election_event.id
 └─ elections[].election_event_id      (one election event, many elections)
     elections[].id
      └─ contests[].election_id        (one election, many contests)
          contests[].id
           └─ candidates[].contest_id  (one contest, many candidates)
```

- Each election, contest and candidate has `tenant_id` and `election_event_id`. Use the values
  of `tenant_id` and `election_event.id` at the top of the file.
- `contests[].election_id` must be the `id` of an election in `elections`.
- `candidates[].contest_id` must be the `id` of a contest in `contests`.
- The links between areas and contests are in `area_contests`. See
  [Election Event File](01-election-event.md).

### IDs

- Each `id` and each reference to an `id` is a UUID version 4, for example
  `3f2b8c1e-5d6a-4b7c-9e8f-0a1b2c3d4e5f`. The import refuses an election, contest or candidate
  whose ID is not a UUID version 4, for example `election-1`.
- The import gives new IDs. It replaces each UUID in the file with a new UUID version 4. The same
  old UUID always gets the same new UUID, so the links stay correct. This also applies to UUIDs
  inside `presentation`, `annotations` and text fields.
- The import puts the election event in the tenant that you are signed in to. It replaces the
  `tenant_id` of the file with the ID of that tenant.

:::caution CAUTION: contest ID of a candidate
Make sure that `candidates[].contest_id` is the `id` of a contest in the file. If it is not a
UUID version 4, the import does not stop: it adds the candidate without a contest, and the
candidate is not on any ballot.
:::

### Values that the import changes

For each election, the import:

- Sets the voting status of the online and kiosk channels to `NOT_STARTED`.
- Clears the start, pause and stop dates of these channels.
- Sets `statistics` to zero and `initialization_report_generated` to `false`.

The import keeps all other fields as they are in the file.

:::caution CAUTION: annotations of contests, candidates and elections
Make sure that each value in `annotations` of a contest, a candidate or an election is text. If a
value is a number, an object or a list, the publication of the ballot fails.
:::

### Common types

| Type | Format |
| --- | --- |
| UUID | A UUID version 4 as text. |
| Date and time | ISO 8601 text with a time zone, for example `2026-05-01T08:00:00Z`. |
| i18n map | An object. Each key is a language code (`en`, `es`, ...). Each value is an object of text fields, for example `{"en": {"name": "Election 1"}, "es": {"name": "Elección 1"}}`. |
| Annotations | An object. **Each value must be text.** To store an object, store it as JSON text. |
| JSON | Any JSON value. |

## Elections

Each object in `elections` is one election.

| Field | Type | Required | Default | Meaning |
| --- | --- | --- | --- | --- |
| `id` | UUID | Yes | - | The ID of the election. |
| `tenant_id` | UUID | Yes | - | The tenant. |
| `election_event_id` | UUID | Yes | - | The election event. |
| `name` | Text | Yes | - | The name of the election. The admin portal copies it from `presentation.i18n.en.name`. |
| `alias` | Text | No | `null` | The external ID of the election, for example the ID in another system. The admin portal shows it as **External ID**. After you set it, the admin portal does not let you change it. |
| `created_at` | Date and time | No | - | Not imported. The import sets the time of the import. |
| `last_updated_at` | Date and time | No | - | Not imported. The import sets the time of the import. |
| `labels` | JSON | No | `null` | Internal. Keep the value from the export. |
| `annotations` | Annotations | No | `null` | Extra data. Each value must be text. |
| `description` | Text | No | `null` | The description. The admin portal copies it from `presentation.i18n.en.description`. |
| `presentation` | Object | No | `null` | The settings of the election in the voting portal: names, languages, ballot design and policies. See [Election Presentation File](09-election-presentation.md). The import refuses a value that is not correct. |
| `status` | Object | No | `null` | The status of the election. See [Election status](#election-status). |
| `eml` | Text | No | `null` | Internal. Keep the value from the export. |
| `num_allowed_revotes` | Integer | No | `null` (= 1) | The number of times that a voter can cast a ballot in the election. `0` means no limit. If the field is empty, the limit is 1. |
| `is_consolidated_ballot_encoding` | Boolean | No | `null` | Internal. Keep the value from the export. |
| `spoil_ballot_option` | Boolean | No | `null` | Internal. Keep the value from the export. |
| `is_kiosk` | Boolean | No | `null` | Internal. Keep the value from the export. |
| `voting_channels` | Object | No | `null` | The voting channels of the election. See [Voting channels](#voting-channels). |
| `image_document_id` | Text | No | `null` | The ID of the image document of the election. In a `.json` file without documents, the image is not in the file. |
| `statistics` | Object | No | - | Reset by the import: `{"num_emails_sent": 0, "num_sms_sent": 0}`. |
| `receipts` | Object | No | `null` | The receipts that voters get. See [Receipts](#receipts). |
| `permission_label` | Text | No | `null` | Only admin portal users with this permission label can see and change the election. |
| `keys_ceremony_id` | UUID | No | `null` | The keys ceremony of the election. Use it only if the file also contains this keys ceremony. |
| `initialization_report_generated` | Boolean | No | - | Reset by the import to `false`. |

The name, the description and the other text of each language are in `presentation.i18n`.

### Election status

All fields are optional. A missing field gets the default value.

| Field | Type | Default | Allowed values and meaning |
| --- | --- | --- | --- |
| `is_published` | Boolean | `false` | `true` if a ballot of the election was published. |
| `voting_status` | Text | `NOT_STARTED` | The status of the online channel: `NOT_STARTED`, `OPEN`, `PAUSED`, `CLOSED`. The import resets it. |
| `kiosk_voting_status` | Text | `NOT_STARTED` | The same values, for the kiosk channel. The import resets it. |
| `voting_period_dates` | Object | empty | The dates of the online channel. The import resets it. |
| `kiosk_voting_period_dates` | Object | empty | The dates of the kiosk channel. The import resets it. |
| `init_report` | Text | `allowed` | `allowed` or `disallowed`. If `disallowed`, you cannot make the initialization report. |
| `allow_tally` | Text | `allowed` | `allowed`, `disallowed` or `requires-voting-period-end`. The **Allow Tally** setting of the election. |

Each dates object has these fields, all optional, as date and time: `first_started_at`,
`last_started_at`, `first_paused_at`, `last_paused_at`, `first_stopped_at`, `last_stopped_at`.

### Voting channels

| Field | Type | Meaning |
| --- | --- | --- |
| `online` | Boolean | Voters can vote in the voting portal. |
| `kiosk` | Boolean | Voters can vote at a kiosk. |

Only `online` and `kiosk` are used in this version. Do not add other channels.

### Receipts

An object with the keys `EMAIL`, `SMS` and `DOCUMENT`. Each value is an object:

| Field | Type | Meaning |
| --- | --- | --- |
| `allowed` | Boolean | The voter can get the receipt by this method. |
| `template` | Text or `null` | The template that the admin portal uses for this method, or `null`. |

## Contests

Each object in `contests` is one contest of an election.

| Field | Type | Required | Default | Meaning |
| --- | --- | --- | --- | --- |
| `id` | UUID | Yes | - | The ID of the contest. |
| `tenant_id` | UUID | Yes | - | The tenant. |
| `election_event_id` | UUID | Yes | - | The election event. |
| `election_id` | UUID | Yes | - | The election of the contest. |
| `created_at`, `last_updated_at` | Date and time | No | - | Not imported. The import sets the time of the import. |
| `labels` | JSON | No | `null` | Internal. Keep the value from the export. |
| `annotations` | Annotations | No | `null` | Extra data. Each value must be text. |
| `is_acclaimed` | Boolean | No | none | A stored flag. The admin portal shows it as **Is Acclaimed**. The voting portal does not use it in this version. |
| `is_active` | Boolean | No | `null` | The admin portal sets `true` for a new contest. Keep the value from the export. |
| `name` | Text | No | `null` | The name of the contest. The text in the presentation is the one that voters see. |
| `description` | Text | No | `null` | The description. The admin portal copies it from `presentation.i18n.en.description`. |
| `presentation` | Object | No | see [Contest presentation](#contest-presentation) | The settings of the contest on the ballot. The import refuses a value that is not correct. |
| `min_votes` | Integer | No | none | The minimum number of selections that the voter must make. The admin portal sets `0` for a new contest. |
| `max_votes` | Integer | No | none | The maximum number of selections that the voter can make. The admin portal sets `1` for a new contest. |
| `winning_candidates_num` | Integer | No | none | The number of winners. The admin portal sets `1` for a new contest. |
| `voting_type` | Text | No | `null` | Only `non-preferential` in the admin portal. The admin portal sets `non-preferential` for a new contest. |
| `counting_algorithm` | Text | Yes | `plurality-at-large` | The counting method. See [Counting algorithms](#counting-algorithms). |
| `is_encrypted` | Boolean | No | `false` | Internal. Keep the value from the export. |
| `tally_configuration` | Object | No | `null` | Tally settings, as a JSON object. Keep the value from the export. |
| `conditions` | JSON | No | `null` | Internal. Keep the value from the export. |
| `image_document_id` | Text | No | `null` | The ID of the image document of the contest. |

:::caution CAUTION: counting algorithm of a contest
Set `counting_algorithm` to `plurality-at-large`. In this version, the tally supports only this
value. If the value is empty or another value, the tally of the election fails.
:::

### Counting algorithms

In this version, the only counting algorithm is `plurality-at-large`. The admin portal offers only
this value. The tally does not support other values.

### Contest presentation

All fields are optional.

| Field | Type | Default | Allowed values and meaning |
| --- | --- | --- | --- |
| `i18n` | i18n map | none | The text of the contest in each language. Keys: `name`, `alias`, `description`. If `alias` is set, the voting portal shows it in place of the name. |
| `allow_writeins` | Boolean | `false` | `true`: the voter can type a name in the write-in candidates of the contest. |
| `base32_writeins` | Boolean | `true` | `true`: write-in text uses a reduced character set (Base32). `false`: a larger character set. |
| `invalid_vote_policy` | Text | `allowed` | What the voting portal does with an invalid ballot. See [Contest policies](#contest-policies). |
| `under_vote_policy` | Text | `allowed` | What the voting portal does if the voter selects fewer than `max_votes`. |
| `blank_vote_policy` | Text | `allowed` | What the voting portal does with a blank vote. |
| `over_vote_policy` | Text | `allowed-with-msg-and-alert` | What the voting portal does if the voter selects more than `max_votes`. The admin portal writes `allowed` for a new contest. |
| `pagination_policy` | Text | `""` | A page name. The voting portal shows the contests with the same value on the same page, in ballot order. Empty: all contests without a value share one page. |
| `cumulative_number_of_checkboxes` | Integer | `1` | The number of points that the voter can give to one candidate. |
| `shuffle_categories` | Boolean | `false` | `true`: the voting portal shows the lists (candidate types) in a random order. `false`: it uses the `sort_order` in `types_presentation`. |
| `shuffle_category_list` | List of text | none | The lists (candidate types) whose candidates the voting portal shows in a random order. |
| `show_points` | Boolean | `false` | `true`: the Ballot Verifier shows the points that the counting algorithm gives to each selection on its confirmation screen. `false`: no points are shown. |
| `enable_checkable_lists` | Text | none (the admin portal writes `allow-selecting-candidates-and-lists`) | What the voter can select when the contest has lists: `allow-selecting-candidates-and-lists`, `allow-selecting-candidates`, `allow-selecting-lists`, `disabled`. |
| `candidates_order` | Text | `alphabetical` | `alphabetical`, `random`, `custom`. With `custom`, the order is the `sort_order` of each candidate. |
| `candidates_selection_policy` | Text | `cumulative` | `radio`: a new selection removes the earlier selection. `cumulative`: selections add up. |
| `candidates_icon_checkbox_policy` | Text | `square-checkbox` | `square-checkbox` or `round-checkbox`. |
| `max_selections_per_type` | Integer | none | The maximum number of selected candidates of the same `type`. |
| `types_presentation` | Object | none | The display of each candidate `type`. See [Types presentation](#types-presentation). |
| `sort_order` | Integer | none | The position of the contest when the election uses the `custom` contest order. |
| `columns` | Integer | `1` | The number of columns of candidates on the ballot. |

The defaults are the values that the system uses when the field is missing. The admin portal
writes most of these fields when you save a contest. If the contest has no `presentation`, write-ins
are not allowed (`allow_writeins` is `false`).

#### Contest policies

| Field | Allowed values |
| --- | --- |
| `invalid_vote_policy` | `allowed`, `warn`, `warn-invalid-implicit-and-explicit`, `not-allowed` |
| `under_vote_policy` | `allowed`, `warn`, `warn-only-in-review`, `warn-and-alert` |
| `blank_vote_policy` | `allowed`, `warn`, `warn-only-in-review`, `not-allowed` |
| `over_vote_policy` | `allowed`, `allowed-with-msg`, `allowed-with-msg-and-alert`, `not-allowed-with-msg-and-alert`, `not-allowed-with-msg-and-disable` |

#### Types presentation

An object. Each key is a candidate `type`. Each value is an object:

| Field | Type | Meaning |
| --- | --- | --- |
| `name` | Text | The name of the type. |
| `name_i18n` | Object | The name in each language: `{"en": "North"}`. |
| `sort_order` | Integer | The position of the type. |
| `subtypes_presentation` | Object | Each key is a candidate `subtype`. Each value has `name`, `name_i18n` and `sort_order`. |

## Candidates

Each object in `candidates` is one candidate. The explicit blank and explicit invalid options and
the write-in fields are also candidates.

| Field | Type | Required | Default | Meaning |
| --- | --- | --- | --- | --- |
| `id` | UUID | Yes | - | The ID of the candidate. |
| `tenant_id` | UUID | Yes | - | The tenant. The import sets it to the tenant of the import. |
| `election_event_id` | UUID | Yes | - | The election event. The import sets it to the election event of the import. |
| `contest_id` | UUID | No | `null` | The contest of the candidate. A candidate without a contest is not on any ballot. |
| `created_at`, `last_updated_at` | Date and time | No | - | Not imported. The import sets the time of the import. |
| `labels` | JSON | No | `null` | Internal. Keep the value from the export. |
| `annotations` | Annotations | No | `null` | Extra data. Each value must be text. |
| `name` | Text | No | `null` | The name of the candidate. The text in the presentation is the one that voters see. |
| `alias` | Text | No | `null` | The alias of the candidate. |
| `description` | Text | No | `null` | The description. |
| `type` | Text | No | `null` | The list (category) of the candidate, as free text. The voting portal shows candidates with the same `type` together. `types_presentation` of the contest uses this value. |
| `presentation` | Object | No | see [Candidate presentation](#candidate-presentation) | The settings of the candidate on the ballot. The import refuses a value that is not correct. |
| `is_public` | Boolean | No | `null` | Internal. Keep the value from the export. |
| `image_document_id` | Text | No | `null` | The ID of the image document of the candidate. |

### Candidate presentation

All fields are optional.

| Field | Type | Default | Allowed values and meaning |
| --- | --- | --- | --- |
| `i18n` | i18n map | none | The text of the candidate in each language. Keys: `name`, `alias`, `description`. |
| `is_explicit_invalid` | Boolean | `false` | `true`: this option makes the ballot invalid on purpose. |
| `is_explicit_blank` | Boolean | `false` | `true`: this option is a blank vote. |
| `is_disabled` | Boolean | `false` | `true`: the voting portal does not show the candidate, for example a withdrawn candidate. |
| `is_category_list` | Boolean | `false` | `true`: this candidate is the header of the list with the same `type`. `enable_checkable_lists` of the contest sets if the voter can select the whole list. |
| `invalid_vote_position` | Text | none | For an explicit invalid option: `top` or `bottom` of the contest. |
| `is_write_in` | Boolean | `false` | `true`: this is a write-in field. The voter types a name. |
| `sort_order` | Integer | none | The position of the candidate when the contest uses the `custom` candidate order. |
| `urls` | List | none | Links of the candidate. See [Candidate URLs](#candidate-urls). |
| `subtype` | Text | none | The subtype of the candidate inside its `type`. |

#### Candidate URLs

Each object in `urls`:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `url` | Text | Yes | The address. |
| `kind` | Text | No | The kind of link. |
| `title` | Text | No | The title of the link. |
| `is_image` | Boolean | Yes | `true`: the voting portal uses this address as the image of the candidate. |

## Example

One election with one contest and two candidates. The other parts of the file are left out. Use
your own UUIDs version 4.

```json
{
  "tenant_id": "0b1c2d3e-4f50-4a61-8b72-c3d4e5f60718",
  "election_event": { "id": "1a2b3c4d-5e6f-4a7b-8c9d-0e1f2a3b4c5d" },
  "elections": [
    {
      "id": "2b3c4d5e-6f70-4b81-9c2d-3e4f5a6b7c8d",
      "tenant_id": "0b1c2d3e-4f50-4a61-8b72-c3d4e5f60718",
      "election_event_id": "1a2b3c4d-5e6f-4a7b-8c9d-0e1f2a3b4c5d",
      "name": "election-1",
      "description": "Election 1",
      "presentation": {
        "i18n": { "en": { "name": "election-1", "alias": "Election 1" } },
        "language_conf": { "enabled_language_codes": ["en"], "default_language_code": "en" }
      },
      "voting_channels": { "online": true, "kiosk": false },
      "num_allowed_revotes": 1,
      "alias": "election-1"
    }
  ],
  "contests": [
    {
      "id": "3c4d5e6f-7081-4c92-8d3e-4f5a6b7c8d9e",
      "tenant_id": "0b1c2d3e-4f50-4a61-8b72-c3d4e5f60718",
      "election_event_id": "1a2b3c4d-5e6f-4a7b-8c9d-0e1f2a3b4c5d",
      "election_id": "2b3c4d5e-6f70-4b81-9c2d-3e4f5a6b7c8d",
      "min_votes": 0,
      "max_votes": 1,
      "winning_candidates_num": 1,
      "voting_type": "non-preferential",
      "counting_algorithm": "plurality-at-large",
      "is_encrypted": true,
      "presentation": {
        "i18n": { "en": { "name": "contest-1", "description": "Contest 1" } },
        "candidates_order": "alphabetical",
        "invalid_vote_policy": "allowed",
        "over_vote_policy": "not-allowed-with-msg-and-alert"
      }
    }
  ],
  "candidates": [
    {
      "id": "4d5e6f70-8192-4da3-9e4f-5a6b7c8d9e0f",
      "tenant_id": "0b1c2d3e-4f50-4a61-8b72-c3d4e5f60718",
      "election_event_id": "1a2b3c4d-5e6f-4a7b-8c9d-0e1f2a3b4c5d",
      "contest_id": "3c4d5e6f-7081-4c92-8d3e-4f5a6b7c8d9e",
      "type": "North",
      "presentation": { "i18n": { "en": { "name": "candidate-1" } } }
    },
    {
      "id": "5e6f7081-92a3-4eb4-8f5a-6b7c8d9e0f1a",
      "tenant_id": "0b1c2d3e-4f50-4a61-8b72-c3d4e5f60718",
      "election_event_id": "1a2b3c4d-5e6f-4a7b-8c9d-0e1f2a3b4c5d",
      "contest_id": "3c4d5e6f-7081-4c92-8d3e-4f5a6b7c8d9e",
      "type": "North",
      "presentation": { "i18n": { "en": { "name": "candidate-2" } } }
    }
  ]
}
```

## If there is a problem

| Problem | Action |
| --- | --- |
| The import fails with `invalid UUID` or `is not v4`. | Make sure that each `id`, `tenant_id`, `election_event_id`, `election_id` and `keys_ceremony_id` is a UUID version 4. |
| The import fails with an error about a `presentation`. | A field of `presentation` has a wrong type or a value that is not allowed. Correct it with the tables on this page. |
| A candidate is not on the ballot. | Make sure that `contest_id` is the `id` of a contest in the file, and that `presentation.is_disabled` is not `true`. |
| The tally of the election fails. | Set `counting_algorithm` of each contest to `plurality-at-large`. |
| The publication fails with an error about annotations. | Make sure that each value in `annotations` is text. |
