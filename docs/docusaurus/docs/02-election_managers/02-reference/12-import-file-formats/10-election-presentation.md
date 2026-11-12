---
id: import_format_election_presentation
title: Election Presentation File
sidebar_position: 10
description: "The JSON file that you can upload in the Data tab of an election to change its presentation settings: how the upload works, every field, the allowed values and examples."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The presentation of an election is a JSON object. It holds the names of the election in each
language and the settings of the election in the voting portal: the ballot design, the policies
and the grace period. Most settings have a field in the **Data** tab of the election. Some settings
do not, for example `voting_period_end` and `css`. To set them, upload an election presentation
file. An election manager does this task.

## How the upload changes the presentation

The admin portal merges the file into the presentation of the election when you click **Save**:

- Each top-level key in the file replaces the same key in the presentation. The admin portal does
  not merge the content of the key. For example, an `i18n` key in the file replaces the names,
  aliases and descriptions in **all** languages.
- The keys that are not in the file do not change.
- The value in the file wins over the value of the same field on the screen.
- The admin portal does not use `language_conf` from the file. It always writes `language_conf`
  from the **Language** section of the screen.
- If you upload a second file before you click **Save**, the admin portal uses only the second
  file.

The admin portal does not check the file. It saves each key and each value as they are in the file.

:::danger WARNING: election presentation file
Use only the keys and the values on this page. Before you save, make sure that each value has the
correct type and is an allowed value. If one value is not correct, the system cannot read the
presentation of the election. Then no voter can cast a ballot in the election, you cannot close the
voting period, and the publication of the ballot fails.
:::

## Before you start

- You have the election manager role and you can edit the election.
- You have a `.json` file. The top level of the file is an object.
- Write characters that are not ASCII (for example `é` or `ñ`) as `\u` escapes: `é`,
  `ñ`. The upload reads the file byte by byte, so other characters show incorrectly.
- Do the change on a test election first.

## Upload the file

1. Open the election in the election event tree.
2. Click the **Data** tab.
3. Click **Advanced Configuration** to open the section.
4. Drag the `.json` file to the upload area, or click the upload area and select the file.
5. Click **Save**.

**Expected result:** The admin portal shows that the election is saved. Open **Advanced
Configuration** again. The **Preview** shows the presentation with the keys of the file.

:::note NOTE: election presentation in the published ballot
The published ballot contains a copy of the election presentation. After you change the
presentation, publish the ballot again so that voters get the change.
:::

## Fields

All fields are optional. The default is the value that the system uses when the field is missing.

| Field | Type | Default | Allowed values and meaning |
| --- | --- | --- | --- |
| `i18n` | Object | none | The text of the election in each language. See [i18n](#i18n). |
| `language_conf` | Object | none | The languages of the election. See [Language configuration](#language-configuration). The upload does not change it. |
| `contests_order` | Text | `alphabetical` | The order of the contests on the ballot. `alphabetical`: by alias, or by name if there is no alias. `random`: a different order for each voter. `custom`: by the `sort_order` of each contest. |
| `audit_button_cfg` | Text | `show` | The **Audit Ballot** button. `show`: shown to the voter. `not-show`: not shown. `show-in-help`: only in the help. |
| `sort_order` | Integer | none | The position of the election in the list of elections, when the election event uses the `custom` election order. |
| `cast_vote_confirm` | Boolean | `false` | `true`: the voter must confirm in a dialog before the ballot is cast. |
| `cast_vote_gold_level` | Text | `no-gold-level` | `gold-level`: the voter must sign in again before the ballot is cast. `no-gold-level`: no new sign-in. |
| `start_screen_title_policy` | Text | `election` | The title and the description on the start screen. `election`: those of the election. `election-event`: those of the election event. |
| `grace_period_policy` | Text | `no-grace-period` | `no-grace-period`: no vote after the scheduled end of voting. `grace-period-without-alert`: a voter who signed in before the scheduled end can cast a ballot for `grace_period_secs` seconds after it. Only for the online channel, and not when voting is paused. |
| `grace_period_secs` | Integer | `0` | The length of the grace period in seconds. Not negative. |
| `voting_period_end` | Text | `allowed` | `allowed`: you can close the voting period. `disallowed`: the system refuses to close it. |
| `initialization_report_policy` | Text | `not-required` | `required`: you cannot open voting before the initialization report is generated. `not-required`: no condition. |
| `security_confirmation_policy` | Text | `none` | `mandatory`: the voter must select a check box with a declaration before voting starts. The text is in `i18n.<language>.security_confirmation_html`. `none`: no check box. |
| `consolidated_report_policy` | Text | `do-not-generate` | `generate`: the tally makes consolidated results for all areas. `do-not-generate`: no consolidated results. |
| `decline_to_vote_policy` | Text | `disabled` | `enabled`: the voter can decline to vote for the whole election. `disabled`: the option is not shown. See [Decline to Vote](../07-decline-to-vote.md). |
| `blank_ballots_policy` | Text | `disabled` | `enabled`: a ballot that is blank in every contest counts as a blank ballot of the election. `disabled`: not counted. See [Blank Ballots](../09-blank-ballots.md). |
| `voting_screen_back_policy` | Text | `election-selection-screen` | Where the **Back** button of the voting screen goes. `election-selection-screen`: the list of elections. `start-screen`: the start screen of the election. |
| `css` | Text | none | Custom CSS for the election on the results website. The voting portal uses the CSS of the election event instead. |
| `custom_filters` | List | none | The custom filters that the admin portal edits in **Advanced Configuration**. The **Users** list reads the filters of the election event, so these filters do not change that list. |

`decline_to_vote_policy` and `blank_ballots_policy` have an effect only when the election event uses
the **Multiple Contests** encryption policy.

### Fields that this version does not use

The presentation can also contain these fields. The system reads them without error, but in this
version it does not use them. Do not add them.

| Field | Type | Allowed values |
| --- | --- | --- |
| `dates` | Object | `start_date` and `end_date`, as date and time text. Set the dates of voting with scheduled events. |
| `is_grace_priod` | Boolean | `true`, `false`. Use `grace_period_policy`. |
| `init_report` | Text | `allowed`, `disallowed` |
| `manual_start_voting_period` | Text | `allowed`, `only-when-initialization-report-has-been-performed` |
| `tally` | Text | `always-allow`, `allow-when-voting-period-ends` |

### i18n

An object. Each key is a language code, for example `en` or `es`. Each value is an object with
these text fields:

| Field | Meaning |
| --- | --- |
| `name` | The name of the election. |
| `alias` | A short name. If it is set, the voting portal shows it in place of the name. |
| `description` | The description of the election. |
| `security_confirmation_html` | The declaration that the voter must accept when `security_confirmation_policy` is `mandatory`. HTML is permitted. |

### Language configuration

| Field | Type | Meaning |
| --- | --- | --- |
| `enabled_language_codes` | List of text | The languages of the election, for example `["en", "es"]`. |
| `default_language_code` | Text | The language to use when the text is not in the language of the voter. If it is not set, the default language of the election event is used. |
| `language_detection_policy` | Text | `browser-detect` or `force-default`. Not used at the election level. The voting portal reads this value from the election event. The admin portal removes it from the election when you save the election. |

The upload does not change `language_conf`. Set the languages in the **Language** section of the
**Data** tab.

## Examples

### Change one setting

This file permits the voting period to close. It does not change other settings.

```json
{
  "voting_period_end": "allowed"
}
```

### Full example

This file sets each field that the upload can change. The `i18n` key replaces the text in all
languages, so it contains all languages of the election.

```json
{
  "i18n": {
    "en": {
      "name": "election-1",
      "alias": "Election 1",
      "description": "Election of the North area",
      "security_confirmation_html": "<p>I confirm that I can vote in this election.</p>"
    },
    "es": {
      "name": "election-1",
      "alias": "Elección 1",
      "description": "Elección del área North",
      "security_confirmation_html": "<p>Confirmo que puedo votar en esta elección.</p>"
    }
  },
  "contests_order": "custom",
  "audit_button_cfg": "show-in-help",
  "sort_order": 1,
  "cast_vote_confirm": true,
  "cast_vote_gold_level": "no-gold-level",
  "start_screen_title_policy": "election",
  "grace_period_policy": "grace-period-without-alert",
  "grace_period_secs": 300,
  "voting_period_end": "allowed",
  "initialization_report_policy": "required",
  "security_confirmation_policy": "mandatory",
  "consolidated_report_policy": "generate",
  "decline_to_vote_policy": "disabled",
  "blank_ballots_policy": "disabled",
  "voting_screen_back_policy": "election-selection-screen",
  "css": "h1 { font-weight: bold; }"
}
```

## If there is a problem

| Problem | Action |
| --- | --- |
| After you save, voters cannot cast a ballot, or the publication fails with `Error parsing election presentation`. | A value in the presentation is not correct. Upload a file that sets the wrong key to a correct value, or open the **Preview** to find it. Then click **Save**. |
| You cannot close the voting period. | Make sure that `voting_period_end` is `allowed` and that all other values are correct. |
| Text shows incorrect characters. | Write characters that are not ASCII as `\u` escapes and upload the file again. |
| The languages did not change. | The upload does not change `language_conf`. Use the **Language** section. |
| The names in other languages disappeared. | The `i18n` key of the file replaced all languages. Upload an `i18n` key that contains all languages. |
| Voters do not see the change. | Publish the ballot again. |
