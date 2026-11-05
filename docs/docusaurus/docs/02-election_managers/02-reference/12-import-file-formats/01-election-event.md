---
id: import_format_election_event
title: Election Event File
sidebar_position: 1
description: "The election event file that you export and import in the admin portal: file types, what the import changes, the JSON structure with all fields, and the contents of the ZIP file."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The election event file contains the configuration of an election event. It can also contain
its voters, logs, keys, publications and results. Use it to make a copy of an election event, or
to move an election event to a different tenant or to a different installation.

Make the file with **Export** on the election event. Import it with **Election Events** > **+** >
**Import Election Event**. Do not make this file yourself, and do not change it. This page
describes the file so that you can examine it and find the cause of an import error.

## File types

| File type | Content |
| --- | --- |
| `.json` | The configuration only: the JSON document that this page describes. |
| `.zip` | A ZIP archive. It contains the JSON document and the other files that you selected in the export. See [ZIP contents](#zip-contents). |
| `.ezip` | A `.zip` file that is encrypted with a password (AES-256-CBC). |

The export always makes a `.zip` or an `.ezip` file. To get a `.json` file, extract the file
`export_election_event-<ID>.json` from the archive.

### File name

The export gives the file this name:

```text
election-event-<election event ID>-export-sha256-<hash>.zip
```

The extension is `.ezip` if the file is encrypted. The `<hash>` is the SHA-256 hash of the
file. The name of a `.json` file is not important. In a `.zip` file, the import looks for an
entry whose name contains `export_election_event` and ends with `.json`. If there is no such
entry, the import stops with "No JSON file found in ZIP".

### Password

The export encrypts the file if you select **Encrypt with Password**, **Bulletin Board**,
**Reports** or **Applications**. The admin portal then shows a password in the **Password**
dialog. Keep the password. You cannot get it again.

To import an `.ezip` file, type the password in **Decryption Password**. The import identifies
an encrypted file by the extension `.ezip`. Do not change the extension.

### Integrity hash

:::caution CAUTION: hash of the election event file
In this version the admin portal does not send the hash of an election event file to the server,
so the import does not check it. Compare the hash yourself before you import: the hash that
`shasum -a 256 <file>` gives must be the same as the hash in the file name. A `.zip` or `.ezip`
export can contain voter data and keys. Keep it in a protected location.
:::

## What the import changes

The import does not copy the election event exactly. It makes these changes:

- It adds the election event to the tenant that you are signed in to. It replaces the
  `tenant_id` of the file everywhere in the file.
- It gives new IDs to the election event and to all its parts. It replaces each ID everywhere
  in the file, so the links between the parts stay correct. The IDs in the settings of the login
  flows (authenticator settings) do not change.
- It resets the `status` of the election event: not published, all voting channels
  `NOT_STARTED`, no voting period dates.
- It resets the voting status and the voting period dates of each election, and sets
  `initialization_report_generated` to `false`.
- It resets the `statistics` of the election event and of each election to 0 emails and
  0 SMS sent.
- It removes the `public_key` of the election event.
- It makes a new bulletin board and a new electoral log for the election event and for each
  election. It replaces `bulletin_board_reference` with the new board. If the ZIP file has the
  electoral log files, the import keeps the exported electoral logs. See
  [Electoral log files](#electoral-log-files).
- It makes new protocol manager keys, unless the ZIP file contains protocol manager keys.
- It makes the login realm of the election event again. See
  [keycloak_event_realm](#keycloak_event_realm).

The import keeps all other values, for example `is_archived`. An election event that was
archived when you exported it is archived after the import.

If the file has no keys ceremony, the election event has no election key after the import. Do a
new keys ceremony.

If the file has bulletin board data, the import keeps the keys ceremonies with their status and execution status. It removes the `public_key` of the election event. Ballot publication reads the election key from the status of the keys ceremony, and the import keeps that status, so a new keys ceremony is not needed for that key.

The import runs as a task. See its result in **Tasks**. If one part of the import fails, the
import stops and the task shows the error. The election event is not added.

## File checks

Before it imports, the system checks the JSON document. It finds all the problems at the same
time. If there is an error, it refuses the file with the message "The election event bundle cannot
be imported; N problems found", and a list of the problems. Each problem gives the path of the
field, for example `contests[2].max_votes`, and the IDs of the file. The admin portal shows the
list when it checks the file.

The check refuses the file if:

- `tenant_id` is empty or is not a UUID, the election event has no `id` or no
  `encryption_protocol`, or there are no elections.
- A contest, a candidate, a link between an area and a contest, or a parent area points at an ID
  that is not in the file. A candidate without `contest_id` is also an error.
- The parent areas make a loop: an area is its own parent, directly or through other areas.
- A contest has no `min_votes`, `max_votes` or `winning_candidates_num`, or one of them is less
  than 0.
- `min_votes` is more than `max_votes`.
- `max_votes` or `winning_candidates_num` is more than the number of candidates of the contest.
- `voting_type` is not `preferential` or `non-preferential`.
- `counting_algorithm` is empty or not an allowed value, or does not agree with `voting_type`. See
  [Counting algorithms](02-election-contest-candidate.md#counting-algorithms).
- Two elections, contests, candidates, areas or links between areas and contests have the same
  `id`.

The check writes a warning in the task log, and imports the file, if:

- There are no areas.
- A contest has no candidates, or no area has the contest on its ballot.
- An area has no contests and is not the parent of another area.
- Elections or reports have permission labels. Only users with these labels can see them.

## Version compatibility

The JSON document has a `version` field: the version of the system that made the export. Before
it imports, the system compares this version with its own version:

| Case | Result |
| --- | --- |
| The major versions are different. | Refused. |
| The minor version of the file is higher than the minor version of the system. | Refused. |
| The minor version of the file is the same or lower. The patch version is not important. | Accepted. |
| The file has the version `dev`, and the system does not. | Refused. |
| The system has the version `dev`. | Accepted, whatever the version of the file. |

The comparison ignores a `v` at the start and a suffix after `-`, for example `-rc.1`. If the file
has no `version` field, the system uses `9.0.0`. Thus versions 10 and later refuse a file without
a `version` field. The error is "Version mismatch: Imported version ... is not compatible with
current version ...".

When you select a file in the import panel, the admin portal checks the file before it imports.
It shows version errors and JSON errors at this step.

## JSON structure

The file is one JSON object. Most values are text, numbers, `true` or `false`, or JSON objects. All
IDs are UUIDs. The import refuses an ID that is not a UUID, and replaces each UUID with a new version 4 UUID.

A field that is "optional" can be absent or `null`. A field that is "required" must be present.

### Top-level object

| Key | Type | Required | Content |
| --- | --- | --- | --- |
| `version` | text | Optional | The version of the system that made the export. Default: `9.0.0`. See [Version compatibility](#version-compatibility). |
| `tenant_id` | UUID | Required | The tenant of the exported election event. The import replaces it with your tenant. |
| `keycloak_event_realm` | object | Optional | The login realm of the election event. See [keycloak_event_realm](#keycloak_event_realm). |
| `election_event` | object | Required | The election event. See [election_event](#election_event). |
| `elections` | array | Required | The elections. See [elections](#elections). Can be empty. |
| `contests` | array | Required | The contests. See [contests](#contests). Can be empty. |
| `candidates` | array | Required | The candidates. See [candidates](#candidates). Can be empty. |
| `areas` | array | Required | The areas. See [areas](#areas). Can be empty. |
| `area_contests` | array | Required | The links between areas and contests. See [area_contests](#area_contests). Can be empty. |
| `scheduled_events` | array | Optional | Scheduled voting periods. See [scheduled_events](#scheduled_events). The export writes `null`. |
| `reports` | array | Required | The reports. See [reports](#reports). The import does not use this array. Can be empty. |
| `keys_ceremonies` | array | Optional | The keys ceremonies. See [keys_ceremonies](#keys_ceremonies). |
| `applications` | array | Optional | The voter applications. See [applications](#applications). |

### election_event

| Field | Type | Required | Default | Content |
| --- | --- | --- | --- | --- |
| `id` | UUID | Required | — | The ID of the election event. The import gives a new ID. |
| `tenant_id` | UUID | Required | — | The tenant. The import replaces it. |
| `created_at` | date and time | Optional | — | Not used. The import sets the time of the import. |
| `updated_at` | date and time | Optional | — | Not used. The import sets the time of the import. |
| `description` | text | Optional | — | The description of the election event. |
| `presentation` | object | Optional | — | The settings and texts of the election event. See [presentation](#election_eventpresentation). |
| `voting_channels` | object | Optional | — | The voting channels. See [voting_channels](#election_eventvoting_channels). |
| `status` | object | Optional | — | The status. The import resets it. See [status](#election_eventstatus). |
| `statistics` | object | Optional | — | The counters of messages sent. The import resets them. See [statistics](#election_eventstatistics). |
| `labels` | object | Optional | — | Free key-value data. See [labels and annotations](#election_eventlabels-and-annotations). |
| `annotations` | object | Optional | — | Free key-value data. See [labels and annotations](#election_eventlabels-and-annotations). |
| `bulletin_board_reference` | object | Optional | — | The bulletin board: `id` (number), `database_name` (text) and `is_archived` (`true` or `false`). All three are required if the object is present. The import replaces it with the new board. |
| `is_archived` | `true` or `false` | Required | — | `true` if the election event is archived. The import keeps the value. |
| `encryption_protocol` | text | Required | — | The encryption protocol. The admin portal always writes `RSA256`. |
| `public_key` | text | Optional | — | The election key. The import removes it. |
| `external_id` | text | Optional | — | The **External ID** of the election event. |
| `is_audit` | `true` or `false` | Optional | — | Internal. Keep the value from the export. |
| `audit_election_event_id` | UUID | Optional | — | Internal. Keep the value from the export. |
| `user_boards` | text | Optional | — | Internal. Keep the value from the export. |

The election event has no `name` field. Its name is in `presentation.i18n.<language>.name`.

#### election_event.presentation

The import checks each field that it knows. If a value is not an allowed value, the import
stops. The import keeps fields that it does not know. The labels in the last column are the labels
in the **Data** tab of the election event. See
[Election Event Data](../02-election-event/03-election_management_election-event_data.md).

| Field | Type | Default | Allowed values | Content |
| --- | --- | --- | --- | --- |
| `i18n` | object | — | `{ "<language>": { "<key>": text or null } }` | The texts in each language. See [i18n](#presentationi18n). |
| `language_conf` | object | — | See [language_conf](#presentationlanguage_conf) | The languages of the election event. |
| `materials` | object | — | `{ "policy": "off" \| "optional" \| "mandatory_for_voting" }` | The support materials policy. Default `off`. |
| `logo_url` | text | — | A URL | **Logo URL**. |
| `redirect_finish_url` | text | — | A URL | **Redirect Finish URL**: the page that the voting portal opens when the voter finishes. |
| `kiosk_redirect_finish_url` | text | — | A URL | **Kiosk Redirect Finish URL**: the same for kiosk voting. |
| `css` | text | — | CSS | **Custom CSS** of the voting portal. |
| `skip_election_list` | `true` or `false` | — | — | **Skip Election List Screen**. |
| `show_user_profile` | `true` or `false` | `true` | — | **Show User Profile**. |
| `show_cast_vote_logs` | text | `hide-logs-tab` | `show-logs-tab`, `hide-logs-tab` | **Show Cast Vote Logs Tab**. |
| `elections_order` | text | `alphabetical` | `alphabetical`, `random`, `custom` | The order of the elections in the voting portal. |
| `voting_portal_countdown_policy` | object | — | See [voting_portal_countdown_policy](#presentationvoting_portal_countdown_policy) | **Voting Portal Countdown policy**. |
| `voting_portal_datetime_format` | text or object | `legacy-gb-24h` | `legacy-gb-24h`, `iso-local`, `us-12h`, `locale-medium`, `date-only`, or `{ "custom": "<pattern>" }` | **Voting Portal date & time format**. A custom pattern uses the tokens `yyyy`, `MM`, `dd`, `HH`, `mm`, `ss`. |
| `custom_urls` | object | — | `{ "login": URL, "enrollment": URL, "saml": URL }`, each optional | Custom URLs for the login, enrollment and SAML pages. |
| `keys_ceremony_policy` | text | `ELECTION_EVENT` | `ELECTION_EVENT`, `ELECTION` | One election key for the election event, or one for each election. |
| `contest_encryption_policy` | text | `single-contest` | `single-contest`, `multiple-contests` | **Contest encryption policy**. |
| `decoded_ballot_inclusion_policy` | text | `not-included` | `included`, `not-included` | **Include decoded ballots to results database**. |
| `locked_down` | text | `not-locked-down` | `locked-down`, `not-locked-down` | **Lockdown Status**. |
| `publish_policy` | text | `always` | `always`, `after-lockdown` | `after-lockdown`: you can publish the election event only after lockdown. |
| `enrollment` | text | `enabled` | `enabled`, `disabled` | **Enrollment** of voters. |
| `otp` | text | `enabled` | `enabled`, `disabled` | **OTP** (one-time code). |
| `voter_signing_policy` | text | `no-signature` | `no-signature`, `with-signature` | **Voter Signing Policy**. |
| `voter_certificate_policy` | text | `disabled` | `disabled`, `enabled` | **Voter Digital Certificate Policy**. |
| `weighted_voting_policy` | text | `disabled-weighted-voting` | `disabled-weighted-voting`, `areas-weighted-voting`, `voters-weighted-voting` | **Weighted Voting Policy**. |
| `delegated_voting_policy` | text | `disabled` | `disabled`, `enabled` | **Delegated Voting Policy**. |
| `ceremonies_policy` | text | `manual-ceremonies` | `manual-ceremonies`, `automated-ceremonies` | **Keys/Tally Ceremonies Policy**. |
| `automatic_recount_policy` | text | `disabled` | `enabled`, `disabled` | **Automatic Recount After Import Approval**. |
| `results_website` | text (JSON) | — | See [results_website](#presentationresults_website) | The results website policy. |
| `custom_filters` | array | — | See below | The custom filters of the voter list in the admin portal. |

Each item of `custom_filters` has a `label` (`name` and an `i18n` object with one text for each
language) and a `filter` (an object of conditions, or `null`). The admin portal writes this
array. Do not change it.

The default applies when the field is absent. If you do not know a value, keep the value of the
export.

#### presentation.i18n

An object with one key for each language code, for example `en` or `es`. Each language has an
object of texts. The admin portal writes these keys:

| Key | Content |
| --- | --- |
| `name` | The name of the election event. |
| `alias` | The alias of the election event. |
| `description` | The description in this language. |
| `materialsTitle` | The title of the support materials. |
| `materialsSubtitle` | The subtitle of the support materials. |

The **Localization** tab adds other keys. Each value is text or `null`. If a text is not
available in a language, the system uses the text in `en`.

#### presentation.language_conf

| Field | Type | Default | Allowed values | Content |
| --- | --- | --- | --- | --- |
| `enabled_language_codes` | array of text | — | Language codes | The languages that voters can select. |
| `default_language_code` | text | `en` | A language code | The default language. |
| `language_detection_policy` | text | `browser-detect` | `browser-detect`, `force-default` | **Language Detection Policy**. With `force-default`, the import sets the default language of the login realm to `default_language_code`. |

#### presentation.voting_portal_countdown_policy

| Field | Type | Allowed values | Content |
| --- | --- | --- | --- |
| `policy` | text | `NO_COUNTDOWN`, `COUNTDOWN`, `COUNTDOWN_WITH_ALERT` | The countdown before the voting session expires. |
| `countdown_anticipation_secs` | whole number | 0 or more | The number of seconds before expiry when the countdown starts. |
| `countdown_alert_anticipation_secs` | whole number | 0 or more | The number of seconds before expiry when the logout alert shows. |

#### presentation.results_website

The value is a JSON object written as text. The import also accepts the object itself. If the
value is present, all three fields are required.

| Field | Allowed values | Default | Content |
| --- | --- | --- | --- |
| `status` | `enabled`, `disabled` | `disabled` | Publish results on the results website. |
| `access` | `public`, `authenticated` | `public` | Who can see the results. |
| `visibility_scope` | `full_event`, `area_based` | `full_event` | All results, or only the results of the area of the user. |

#### election_event.voting_channels

| Field | Type | Content |
| --- | --- | --- |
| `online` | `true` or `false` | Online voting is on. |
| `kiosk` | `true` or `false` | Kiosk voting is on. |
| `telephone` | `true` or `false` | Telephone voting is on. |
| `paper` | `true` or `false` | Paper voting is on. |
| `early_voting` | `true` or `false` | Early voting is on. |

All fields are optional. A new election event has `online` set to `true`.

#### election_event.status

The import replaces this object with a new status. The values in the file are not used. Each
field is optional.

| Field | Type | Allowed values | Content |
| --- | --- | --- | --- |
| `is_published` | `true` or `false` | — | The election event is published. After the import: `false`. |
| `voting_status` | text | `NOT_STARTED`, `OPEN`, `PAUSED`, `CLOSED` | The status of online voting. After the import: `NOT_STARTED`. |
| `kiosk_voting_status` | text | Same | The status of kiosk voting. |
| `early_voting_status` | text | Same | The status of early voting. |
| `telephone_voting_status` | text | Same | The status of telephone voting. |
| `voting_period_dates` | object | See below | The dates of the online voting period. |
| `kiosk_voting_period_dates` | object | See below | The dates of the kiosk voting period. |
| `early_voting_period_dates` | object | See below | The dates of the early voting period. |
| `telephone_voting_period_dates` | object | See below | The dates of the telephone voting period. |

Each `*_period_dates` object has the optional fields `first_started_at`, `last_started_at`,
`first_paused_at`, `last_paused_at`, `first_stopped_at` and `last_stopped_at`. Each is a date and
time in UTC, in ISO 8601 format.

#### election_event.statistics

| Field | Type | Content |
| --- | --- | --- |
| `num_emails_sent` | whole number | The number of emails sent. After the import: 0. |
| `num_sms_sent` | whole number | The number of SMS sent. After the import: 0. |

#### election_event.labels and annotations

Two JSON objects with free keys and values. The admin portal does not show them. Some
integrations keep data here. Keep both objects as the export wrote them.

### elections

The elections of the election event. See
[Elections, Contests and Candidates](02-election-contest-candidate.md).

### contests

The contests of the elections. See
[Elections, Contests and Candidates](02-election-contest-candidate.md).

### candidates

The candidates of the contests. See
[Elections, Contests and Candidates](02-election-contest-candidate.md).

### areas

One object for each area.

| Field | Type | Required | Content |
| --- | --- | --- | --- |
| `id` | UUID | Required | The ID of the area. The import gives a new ID. |
| `tenant_id` | UUID | Required | The tenant. The import replaces it. |
| `election_event_id` | UUID | Required | The election event. The import replaces it. |
| `created_at` | date and time | Optional | Not used. The import sets the time of the import. |
| `last_updated_at` | date and time | Optional | Not used. The import sets the time of the import. |
| `name` | text | Optional | The name of the area, for example `North`. The voter file uses this name in `area_name`. |
| `description` | text | Optional | The description of the area. |
| `type` | text | Optional | The type of the area. Free text. |
| `parent_id` | UUID | Optional | The ID of the parent area. The parent must be in the same file. |
| `presentation` | object | Optional | See below. |
| `labels` | object | Optional | Free key-value data. |
| `annotations` | object | Optional | See below. |

The area `presentation` object:

| Field | Type | Default | Allowed values | Content |
| --- | --- | --- | --- | --- |
| `allow_early_voting` | text | `no_early_voting` | `allow_early_voting`, `no_early_voting` | Voters of this area can vote early. |

The area `annotations` object can contain:

| Field | Type | Default | Allowed values | Content |
| --- | --- | --- | --- | --- |
| `weight` | whole number | 1 | 0 or more | The weight of the votes of this area, when `weighted_voting_policy` is `areas-weighted-voting`. |
| `tally_operation` | text | — | `process-ballots-all`, `aggregate-results`, `skip-candidate-results` | How the tally treats this area. |

### area_contests

One object for each link between an area and a contest.

| Field | Type | Required | Content |
| --- | --- | --- | --- |
| `id` | UUID | Required | The ID of the link. The import gives a new ID. |
| `area_id` | UUID | Required | The area. It must be in `areas`. |
| `contest_id` | UUID | Required | The contest. It must be in `contests`. |

### keys_ceremonies

The keys ceremonies of the election event. The export fills this array only if you select
**Bulletin Board**. Otherwise it writes an empty array.

| Field | Type | Required | Allowed values | Content |
| --- | --- | --- | --- | --- |
| `id` | UUID | Required | — | The ID of the keys ceremony. The import gives a new ID. |
| `tenant_id` | UUID | Required | — | The tenant. The import replaces it. |
| `election_event_id` | UUID | Required | — | The election event. The import replaces it. |
| `created_at` | date and time | Optional | — | Not used. |
| `last_updated_at` | date and time | Optional | — | Not used. |
| `trustee_ids` | array of text | Required | Trustee names | The **names** of the trustees, not their IDs. See the caution below. |
| `threshold` | whole number | Required | 1 or more | The number of trustees necessary for the tally. |
| `name` | text | Optional | — | The name of the keys ceremony. |
| `status` | object | Optional | See below | The progress of the ceremony. The import keeps it. |
| `execution_status` | text | Optional | `USER_CONFIGURATION`, `STARTED`, `IN_PROGRESS`, `SUCCESS`, `CANCELLED` | The status of the ceremony. The import keeps it. |
| `settings` | object | Optional | `{ "policy": "manual-ceremonies" \| "automated-ceremonies" }` | The ceremonies policy of the ceremony. |
| `is_default` | `true` or `false` | Optional | — | The ceremony is the default ceremony of the election event. If absent, the import uses `false`. |
| `permission_label` | array of text | Optional | — | The permission labels of the ceremony. |
| `labels` | object | Optional | — | Not imported. |
| `annotations` | object | Optional | — | Not imported. |

The `status` object has these fields: `stop_date` (text), `public_key` (text), `logs` (an array of
`{ "created_date": text, "log_text": text }`) and `trustees` (an array of
`{ "name": text, "status": "WAITING" | "KEY_GENERATED" | "KEY_RETRIEVED" | "KEY_CHECKED" }`).

:::caution CAUTION: trustees in the election event file
Make sure that the tenant where you import has trustees with the same names as the trustees in
`trustee_ids`. The import finds each trustee by its name. If a name is not found, the import
fails with "Error parsing trustee_ids as UUIDs".
:::

### applications

The voter applications of the election event. The export fills this array only if you select
**Applications**. The fields are the same as in the applications file. See
[Applications](05-applications.md).

### scheduled_events

The export writes `null`. The scheduled events of an export are in the CSV file
`export_scheduled_events-<ID>.csv` in the ZIP file. See
[Scheduled events file](#scheduled-events-file).

If the JSON document contains this array, the import uses only the items whose `event_processor`
is `START_VOTING_PERIOD` or `END_VOTING_PERIOD`. It adds them only if `tenant_id`,
`election_event_id` and `task_id` are correct for the imported election event, and if
`cron_config.scheduled_date` has a value. It ignores all other items.

| Field | Type | Required | Content |
| --- | --- | --- | --- |
| `id` | UUID | Required | The ID of the scheduled event. |
| `tenant_id` | UUID | Optional | The tenant. |
| `election_event_id` | UUID | Optional | The election event. |
| `created_at` | date and time | Optional | When the scheduled event was made. |
| `stopped_at` | date and time | Optional | When the scheduled event was stopped. |
| `archived_at` | date and time | Optional | When the scheduled event was archived. |
| `event_processor` | text | Optional | The action. See the values below. |
| `cron_config` | object | Optional | `{ "cron": text, "scheduled_date": text }`. `scheduled_date` is the date and time of the action, in ISO 8601 format. |
| `event_payload` | object | Optional | For a voting period: `{ "election_id": UUID or null, "voting_channels": [ ... ] }`. `null` means the election event. The channels are `ONLINE`, `KIOSK`, `EARLY_VOTING`, `TELEPHONE`. If there are no channels, the action applies to `ONLINE` and `KIOSK`. |
| `task_id` | text | Optional | The internal name of the scheduled task. |
| `labels` | object | Optional | Free key-value data. |
| `annotations` | object | Optional | Free key-value data. |

The values of `event_processor` are: `ALLOW_INIT_REPORT`, `CREATE_REPORT`, `SEND_TEMPLATE`,
`START_VOTING_PERIOD`, `END_VOTING_PERIOD`, `ALLOW_VOTING_PERIOD_END`,
`START_ENROLLMENT_PERIOD`, `END_ENROLLMENT_PERIOD`, `START_LOCKDOWN_PERIOD`,
`END_LOCKDOWN_PERIOD` and `ALLOW_TALLY`.

### reports

The export fills this array only if you select **Reports**. The import does not use this array.
It reads the reports from the CSV file `export_reports-<ID>.csv` in the ZIP file. See
[Reports file](#reports-file). The array must be present, but it can be empty.

| Field | Type | Required | Content |
| --- | --- | --- | --- |
| `id` | UUID | Required | The ID of the report. |
| `tenant_id` | UUID | Required | The tenant. |
| `election_event_id` | UUID | Required | The election event. |
| `election_id` | UUID | Optional | The election of the report. `null` for the election event. |
| `report_type` | text | Required | The type of report, for example `ELECTORAL_RESULTS`. |
| `template_alias` | text | Optional | The alias of the template of the report. |
| `encryption_policy` | text | Required | `unencrypted` or `configured_password`. |
| `cron_config` | object | Optional | The schedule of the report. See [Reports file](#reports-file). |
| `created_at` | date and time | Required | When the report was made. |
| `permission_label` | array of text | Optional | The permission labels of the report. |

The values of `report_type` are: `INITIALIZATION_REPORT`, `ELECTORAL_RESULTS`, `BALLOT_IMAGES`,
`BALLOT_RECEIPT`, `ACTIVITY_LOGS`, `MANUAL_VERIFICATION`, `PARTICIPATION_REPORT` and
`CREDENTIALS`.

### keycloak_event_realm

The login realm of the election event. It contains the login configuration of the voters: the
login pages and their texts, the languages, the voter fields (user profile), the login flows and
their settings, the identity providers and the clients. The export always writes it.

The import makes the login realm of the new election event from this object:

- It gives the realm the name `tenant-<tenant ID>-event-<election event ID>`.
- It sets the URLs of the voting portal, the kiosk voting portal and the results website to the
  URLs of the installation where you import.
- It removes the client secrets. The system makes new secrets, or uses the secrets that the
  installation configures.
- It removes the signing keys. It keeps only their algorithm and priority, so the system makes
  new keys.
- If `language_detection_policy` is `force-default`, it sets the default language of the realm
  to `default_language_code`.

If the object is absent, the import uses the default login realm of the installation. Do not
change this object. To change the login configuration, use the admin portal after the import.

## Example

### Minimal file

This file makes an election event with one area and no elections. It has no
`keycloak_event_realm`, so the import uses the default login realm.

```json
{
  "version": "10.1.0",
  "tenant_id": "11111111-1111-4111-8111-111111111111",
  "election_event": {
    "id": "22222222-2222-4222-8222-222222222222",
    "tenant_id": "11111111-1111-4111-8111-111111111111",
    "is_archived": false,
    "encryption_protocol": "RSA256",
    "presentation": {
      "i18n": { "en": { "name": "Election Event 1" } },
      "language_conf": {
        "enabled_language_codes": ["en"],
        "default_language_code": "en"
      }
    },
    "voting_channels": { "online": true }
  },
  "elections": [],
  "contests": [],
  "candidates": [],
  "areas": [
    {
      "id": "33333333-3333-4333-8333-333333333333",
      "tenant_id": "11111111-1111-4111-8111-111111111111",
      "election_event_id": "22222222-2222-4222-8222-222222222222",
      "name": "North",
      "description": "Area North",
      "presentation": { "allow_early_voting": "no_early_voting" }
    }
  ],
  "area_contests": [],
  "reports": []
}
```

### election_event with presentation

A fuller `election_event` object, as the export writes it (shortened):

```json
{
  "id": "22222222-2222-4222-8222-222222222222",
  "tenant_id": "11111111-1111-4111-8111-111111111111",
  "created_at": "2026-09-01T10:00:00+00:00",
  "updated_at": "2026-09-15T08:30:00+00:00",
  "description": "Election event for the annual election",
  "external_id": "event-1",
  "is_archived": false,
  "encryption_protocol": "RSA256",
  "public_key": null,
  "labels": {},
  "annotations": {},
  "voting_channels": {
    "online": true,
    "kiosk": false,
    "telephone": false,
    "paper": false,
    "early_voting": false
  },
  "status": {
    "is_published": false,
    "voting_status": "NOT_STARTED",
    "kiosk_voting_status": "NOT_STARTED",
    "early_voting_status": "NOT_STARTED",
    "telephone_voting_status": "NOT_STARTED",
    "voting_period_dates": {},
    "kiosk_voting_period_dates": {},
    "early_voting_period_dates": {},
    "telephone_voting_period_dates": {}
  },
  "statistics": { "num_emails_sent": 0, "num_sms_sent": 0 },
  "presentation": {
    "i18n": {
      "en": {
        "name": "Election Event 1",
        "alias": "EE1",
        "description": "Annual election",
        "materialsTitle": "Support materials",
        "materialsSubtitle": "Read before you vote"
      },
      "es": {
        "name": "Evento Electoral 1",
        "alias": "EE1",
        "description": "Elección anual"
      }
    },
    "language_conf": {
      "enabled_language_codes": ["en", "es"],
      "default_language_code": "en",
      "language_detection_policy": "browser-detect"
    },
    "materials": { "policy": "optional" },
    "logo_url": "https://example.com/logo.png",
    "redirect_finish_url": "https://example.com/thank-you",
    "css": "",
    "skip_election_list": false,
    "show_user_profile": true,
    "show_cast_vote_logs": "hide-logs-tab",
    "elections_order": "alphabetical",
    "voting_portal_countdown_policy": {
      "policy": "COUNTDOWN_WITH_ALERT",
      "countdown_anticipation_secs": 60,
      "countdown_alert_anticipation_secs": 30
    },
    "voting_portal_datetime_format": "iso-local",
    "keys_ceremony_policy": "ELECTION_EVENT",
    "contest_encryption_policy": "single-contest",
    "decoded_ballot_inclusion_policy": "not-included",
    "locked_down": "not-locked-down",
    "publish_policy": "always",
    "enrollment": "disabled",
    "otp": "enabled",
    "voter_signing_policy": "no-signature",
    "voter_certificate_policy": "disabled",
    "weighted_voting_policy": "disabled-weighted-voting",
    "delegated_voting_policy": "disabled",
    "ceremonies_policy": "manual-ceremonies",
    "automatic_recount_policy": "disabled",
    "results_website": "{\"status\":\"enabled\",\"access\":\"public\",\"visibility_scope\":\"full_event\"}"
  }
}
```

## ZIP contents

:::caution CAUTION: files in the election event ZIP archive
Do not edit the files in the ZIP archive. They come from an export, and the import expects them
exactly as the export wrote them. A changed file can stop the import, or make an election event
with wrong data.
:::

The import finds each entry by a part of its name. `<ID>` is the ID of the exported election
event. The export option is the check box in the **Export Election Event** dialog.

| Entry | Optional | Content | Export option |
| --- | --- | --- | --- |
| `export_election_event-<ID>.json` | No | The JSON document. See [JSON structure](#json-structure). | Always |
| `images/document_<document ID>_<name>` | Yes | The images of the elections, contests and candidates. The import adds them as public documents with new IDs. | Always, if there are images |
| `export_voters-<ID>.csv` | Yes | The voters. See [Voters file](#voters-file). | **Include Voters** |
| `export_reports-<ID>.csv` | Yes | The reports. See [Reports file](#reports-file). | **Reports** |
| `export_activity_logs-<ID>` | Yes | The electoral log in CSV format. The entry has no extension. If the ZIP file has no electoral log files, the import adds the log messages to the electoral log of the new election event. | **Activity Logs** |
| `electoral_log_records-<ID>.jsonl` | Yes | The records of the electoral logs. See [Electoral log files](#electoral-log-files). | **Activity Logs** |
| `electoral_log_manifest-<ID>.json` | Yes | The identity, size, root and published checkpoints of each electoral log. See [Electoral log files](#electoral-log-files). | **Activity Logs** |
| `export_S3_files/document_<document ID>_<name>` | Yes | The private documents of the election event. The import adds them with new IDs. | **S3 Files** |
| `export_scheduled_events-<ID>.csv` | Yes | The scheduled events. See [Scheduled events file](#scheduled-events-file). | **Scheduled Events** |
| `export_publications-<ID>.json` | Yes | The ballot publications. | **Publications** |
| `publication_files/<publication ID>/<file>.json` | Yes | The files of each publication: `event.json`, `election-<ID>.json`, `style-<ID>.json` and `summary-<ID>.json`. | **Publications** |
| `election_event_config-<ID>.json` | Yes | The public configuration of the election event that a publication makes. | **Publications**, if there is a publication |
| `export_protocol_manager_keys-<ID>.csv` | Yes | The protocol manager keys. See [Protocol manager keys file](#protocol-manager-keys-file). | **Bulletin Board**, **Activity Logs** or **Publications** |
| `export_bulletin_boards-<ID>.csv` | Yes | The messages of the bulletin boards. See [Bulletin boards file](#bulletin-boards-file). | **Bulletin Board**, if there is a keys ceremony |
| `export_tally_data/<name>.csv` | Yes | The tally sessions and the results. See [Tally data files](#tally-data-files). | **Tally** (the admin portal also selects **Bulletin Board**) |
| `export_certificates-<ID>.pem` | Yes | The CA certificates of the election event, in PEM format. The import adds each certificate. | **Certificates**, if there are certificates |

The import reads the protocol manager keys first. Then it reads the other entries in alphabetical
order, and the tally data files last.

Values in the CSV files that hold JSON (objects, arrays, quoted text) are written as JSON. For
example, a text value is written with its quotes: `"START_VOTING_PERIOD"`.

### Electoral log files

With **Activity Logs**, the export also writes the electoral logs of the election event with their
roots:

- `electoral_log_records-<ID>.jsonl`: the records of each electoral log, in log order. Each line
  is one JSON object.
- `electoral_log_manifest-<ID>.json`: the identity, the size, the root and the published
  checkpoints of each electoral log.

If the ZIP file has the two files, the import stores the electoral logs with the same records and
the same roots. It refuses the logs if a root is not the same as its published checkpoint. The
new bulletin board of the election event continues from the exported board. The import then does
not use `export_activity_logs-<ID>`.

:::caution CAUTION: keep the two electoral log files together
Do not remove or change one of the two files. If the ZIP file has only one of them, the import
fails with "The export has the electoral-log manifest but not its records" or "The export has the
electoral-log records but not their manifest".
:::

### Voters file

`export_voters-<ID>.csv`. The import reads it as a voter import of the new election event. See
[Voters](03-voters.md).

| Column | Content |
| --- | --- |
| `id` | The ID of the voter. The import does not use it. Each voter gets a new ID. |
| `email` | The email address, for example `voter001@example.com`. |
| `email_verified` | `true` or `false`. |
| `enabled` | `true` or `false`. |
| `first_name` | The first name. |
| `last_name` | The last name. |
| `username` | The user name. |
| `area_name` | The name of the area of the voter, or `-` if the voter has no area. |
| `<attribute>` | One column for each voter field of the login realm. Secret voter fields are present only if you selected **Encrypt with Password** and you have the permission to read secret voter fields. |
| `election__<election alias or name>` | One column for each election: the date and time when the voter last voted, or empty. The import does not use these columns. |

The file contains no passwords.

### Reports file

`export_reports-<ID>.csv`. One row for each report.

| Column | Content |
| --- | --- |
| `ID` | The ID of the report. The import does not use it. Each report gets a new ID. |
| `Election ID` | The election of the report, or empty for the election event. |
| `Report Type` | The type of report. See [reports](#reports). |
| `Template Alias` | The alias of the template, or empty. |
| `Cron Config` | The schedule of the report as JSON: `is_active` (`true` or `false`), `cron_expression` (text), `email_recipients` (array of email addresses), `executer_username` (text), `last_document_produced` (text). Or `null`. |
| `Encryption Policy` | `unencrypted` or `configured_password`. |
| `Password` | The password of an encrypted report, as plain text, or empty. |
| `Permission Labels` | The permission labels, separated with `\|`, or empty. |

:::danger WARNING: election event export with reports
Keep an export with reports in a protected location. The reports file contains the passwords of
the encrypted reports as plain text. For this reason the export always encrypts the archive when
you select **Reports**.
:::

### Scheduled events file

`export_scheduled_events-<ID>.csv`. One row for each scheduled event. The columns are the fields
of [scheduled_events](#scheduled_events), in this order:

`id`, `tenant_id`, `election_event_id`, `created_at`, `stopped_at`, `archived_at`, `labels`,
`annotations`, `event_processor`, `cron_config`, `event_payload`, `task_id`

The import:

- Gives each scheduled event a new ID, and does not use `id`, `tenant_id` and
  `election_event_id`.
- Replaces the `election_id` in `event_payload` with the new ID of the election.
- Makes a new `task_id`.
- Clears `stopped_at` and `archived_at`. All imported scheduled events are active.

### Protocol manager keys file

`export_protocol_manager_keys-<ID>.csv`. The secret keys of the bulletin boards.

| Column | Content |
| --- | --- |
| `election_id` | Empty for the board of the election event. Otherwise the ID of the election of the board. |
| `key` | The protocol manager key of the board. |

The file must have one row for the election event and one row for each election. If a row is
missing, the import fails. If this file is present, the import does not make new protocol manager
keys.

:::danger WARNING: protocol manager keys file
Keep this file secret. The protocol manager keys protect the bulletin boards of the election
event.
:::

### Bulletin boards file

`export_bulletin_boards-<ID>.csv`. One row for each message of the bulletin boards.

| Column | Content |
| --- | --- |
| `election_id` | Empty for the board of the election event. Otherwise the ID of the election of the board. |
| `id` | The number of the message in the board. |
| `created` | When the message was made (a timestamp). |
| `sender_pk` | The public key of the sender of the message. |
| `statement_timestamp` | The timestamp of the statement in the message. |
| `statement_kind` | The kind of statement. |
| `batch` | The batch number. |
| `mix_number` | The mix number. |
| `message` | The message, in Base64 without padding. |
| `version` | The version of the message format. |

### Tally data files

The folder `export_tally_data/` contains ten CSV files. The import reads
`export_tally_session.csv` and `export_results_event.csv` before the other files.

These columns are in most files:

| Column | Content |
| --- | --- |
| `id` | The ID of the row. The import gives a new ID. |
| `tenant_id`, `election_event_id` | The tenant and the election event. The import replaces them. |
| `election_id`, `contest_id`, `area_id`, `candidate_id` | The election, contest, area or candidate of the row. The import replaces them with the new IDs. |
| `results_event_id` | The results of the tally session. |
| `created_at`, `last_updated_at` | When the row was made and last changed. |
| `labels`, `annotations` | Free key-value data as JSON. |
| `documents` | The documents of the results (for example PDF, HTML, JSON) as JSON. |
| `name` | The name of the results or of the area. |

The files and their other columns:

| File | Content | Other columns |
| --- | --- | --- |
| `export_tally_session.csv` | The tally sessions. | `election_ids`, `area_ids`, `is_execution_completed`, `keys_ceremony_id`, `execution_status`, `threshold`, `configuration`, `tally_type` (`ELECTORAL_RESULTS` or `INITIALIZATION_REPORT`), `permission_label` |
| `export_tally_session_contest.csv` | The contests and areas of each tally session. | `session_id`, `tally_session_id` |
| `export_tally_session_execution.csv` | The steps of each tally session. | `current_message_id`, `tally_session_id`, `session_ids`, `status` |
| `export_results_event.csv` | One row for each set of results. | — |
| `export_results_election.csv` | The results of each election. | `elegible_census`, `total_voters`, `total_voters_percent` |
| `export_results_election_area.csv` | The results of each election in each area. | — |
| `export_results_contest.csv` | The results of each contest. | `elegible_census`, `total_votes`, `total_valid_votes`, `total_invalid_votes`, `explicit_invalid_votes`, `implicit_invalid_votes`, `total_blank_votes`, `explicit_blank_votes`, `implicit_blank_votes`, `total_auditable_votes`, the `_percent` column of each count, `voting_type`, `counting_algorithm` |
| `export_results_area_contest.csv` | The results of each contest in each area. | The same counts and `_percent` columns as `export_results_contest.csv` |
| `export_results_contest_candidate.csv` | The results of each candidate. | `cast_votes`, `cast_votes_percent`, `winning_position`, `points` |
| `export_results_area_contest_candidate.csv` | The results of each candidate in each area. | `cast_votes`, `cast_votes_percent`, `winning_position`, `points` |

The `_percent` columns are the same counts as a percentage. `elegible_census` is the number of
voters who can vote.

## Related pages

- [Create the Election Event](../../03-procedures/02-event.md)
- [Import File Formats](index.md)
