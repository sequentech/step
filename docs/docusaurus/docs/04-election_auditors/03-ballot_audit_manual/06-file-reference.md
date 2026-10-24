---
title: File Reference
sidebar_position: 6
description: "The content of each file that the ballot audit commands write, and which files contain personal data."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# File Reference

This page gives the content of each file that the commands write. The directory names are the
names that the procedures use. `audit-tally` writes the same files of each command in the
directories of [Files of audit-tally](#files-of-audit-tally-tally).

## Common rules

- The files are JSON, except `configuration.borsh`. A JSON object that has `format_version`
  uses version `1`.
- A **message** is the original signed bytes of a bulletin board or electoral log record,
  encoded in base64 without padding. The commands check the signature of each message. Do not
  change a message.
- **Personal data** in the tables shows if the file contains voter IDs, hashed voter identifiers,
  user names or receipts. Keep these files private. See
  [Report and Remove the Copies](02-procedures/04-close.md).
- Fields of the `limitations`, `limits`, `limitation` and `assurance` types are text. They tell
  what the command did not check. Copy them into the audit report.

:::danger WARNING
Do not edit a file and then use it again. A change of one byte changes its digest, and the next
command stops with a digest mismatch.
:::

## Files of the audit image (backups and output)

The audit image reads the backups directory at `/backups` and writes to the output directory at
`/output`. The procedures use `~/audit/backups` and `~/audit/output`:

```text
~/audit/
├── audit-notes.txt          you write it
├── backups/                 read only for the audit
│   ├── backup-digests.txt   you write it
│   ├── hasura.dump, keycloak.dump, b3.dump, immudb.tar.gz
│   └── b3-blobs.tar.gz      only for a board that stores messages in files
└── output/                  must be empty before the audit
    ├── restore/             the messages of the restore
    └── the files of audit-tally
```

| File in `output/restore` | Content | Personal data |
| --- | --- | --- |
| `restore-hasura.log`, `restore-keycloak.log`, `restore-b3.log` | The messages of `pg_restore` for each copy. | No |
| `immudb-grants.log` | The messages of the read permissions on each ImmuDB database. A failure on a name that starts with a digit is expected: no audited board has such a name. | No |
| `postgres-init.log`, `immudb.log`, `b3-server.log` | The messages of the database servers of the copies. `b3-server.log` shows the password of the PostgreSQL copy, which the container removed. | No |

The copies are inside the container. Docker removes them when the command stops. The files of
`output` belong to the owner of `backup-digests.txt`.

## Files of audit-tally (tally)

`audit-tally` writes all files of the audit in its output directory. With the audit image, it is
`~/audit/output`:

```text
tally/
├── tally-plan.json
├── immudb-export/              only without --electoral-log-export
├── scopes/
│   ├── 001/
│   │   ├── scope.json
│   │   ├── postgres-export/
│   │   ├── prepared-audit/
│   │   └── audit-report.json
│   └── 002/ ...
├── history/
│   └── <election ID>.json
├── braid/
│   └── verify.log
├── audit-record.json
└── public-summary.json
```

| File or directory | Content | Personal data |
| --- | --- | --- |
| `tally-plan.json` | The scopes that the command made from the tally. See [tally-plan.json](#tally-planjson). | No |
| `immudb-export/` | The files of `export-electoral-log`. See [Files of export-electoral-log](#files-of-export-electoral-log-immudb-export). | Yes |
| `scopes/<NNN>/scope.json` | The scope of an election and area, with one contest or all contests for combined ballot formats. See [scope.json](05-command-reference.md#scopejson). | No |
| `scopes/<NNN>/postgres-export/` | The files of `export-ballot-audit` for the scope. | Yes |
| `scopes/<NNN>/prepared-audit/` | The audit bundle of the scope. | Yes |
| `scopes/<NNN>/audit-report.json` | The report of `ballot-audit` for the scope. | Yes |
| `history/<election ID>.json` | The report of `audit-cast-history` for one election. | Yes |
| `braid/verify.log` | The output of the Braid verifier. | No |
| `audit-record.json` | The record of the audit. See [audit-record.json](#audit-recordjson). | No |
| `public-summary.json` | The summary that you can publish. See [public-summary.json](#public-summaryjson). | No |

The scopes are numbered from `001`, in the order of election, area and contest.

### tally-plan.json

| Field | Content |
| --- | --- |
| `tenant_id`, `election_event_id`, `tally_session_id` | The tally. |
| `board_name` | The name of the event board. |
| `policy` | `unweighted`, `weighted` or `delegated`, from the configuration of the tally. |
| `scopes[]` | One object for each scope: `directory`, `election_id`, `area_id`, `contest_id`, `tally_session_contest_id`, `base_batch`, `batches` and `scope_sha512`, the digest of its `scope.json`. |
| `tally_batches` | All batches of the scopes. |
| `board_ballot_batches` | All batches with `Ballots` statements on the board. |
| `hasura_snapshot`, `b3_snapshot` | The database, the restore reference, the time of the read and the read-only settings of each read. |

### audit-record.json

| Field | Content |
| --- | --- |
| `source_revision` | The source code revision of the command. |
| `started_at`, `finished_at` | The time of the start and of the end of the audit. |
| `inputs` | The tally ID, the two keys, the restore reference of each copy, and the addresses and directories that you gave. |
| `tally_plan`, `public_summary` | The digest (`sha512`) and the size (`bytes`) of `tally-plan.json` and of `public-summary.json`. |
| `status` | The status of the full audit. See [Read the Results](03-read-the-results.md#statuses-of-audit-tally). |
| `steps[]` | One object for each step. See the table below. |
| `braid_batches` | The batches that the command gave to the Braid verifier. |
| `limitations` | Text. What the audit does not prove. |

Each object of `steps` has these fields:

| Field | Content |
| --- | --- |
| `step` | The command: `export-electoral-log`, `export-ballot-audit`, `prepare-ballot-audit`, `ballot-audit`, `audit-cast-history` or `verify`. |
| `scope` | The directory of the scope, for example `scopes/001`. Only for the steps of a scope. |
| `election_id` | The election. Only for `audit-cast-history`. |
| `command` | The command line. Paths in the output directory are relative to it. Empty if the step did not run. |
| `exit_code` | The exit status of the command. `null` if the command did not start or did not run. |
| `status` | The status of the step. |
| `outputs` | The digest of each file that the step wrote, by path. |
| `message` | The messages of the command, or the cause of `not_run`. Only if there is a message. |

:::note
`audit-record.json` contains the digests of all files of the audit. Record its digest in the
audit notes.
:::

### public-summary.json

The summary contains only IDs of the tally, the board name, statuses, counts and batch numbers.
The command removes all text from the selection counts. The summary contains no voter IDs, no
receipts and no batch positions.

| Field | Content |
| --- | --- |
| `source_revision` | The source code revision of the command. |
| `tenant_id`, `election_event_id`, `tally_session_id`, `policy` | The tally. |
| `status` | The status of the full audit. |
| `scopes[]` | For each scope: `election_id`, `area_id`, `contest_id`, `batches`, `status`, `checked_cast_votes`, `checked_batches`, `checked_electoral_log_messages`, the number of `duplicate_selected_receipts` and of `privacy_warnings`, and `selection_accounting`, the counts of the selection. |
| `cast_vote_history[]` | For each election: `election_id`, `status`, `outcome`, `checked_hasura_rows`, `checked_signed_cast_vote_log_records`, `signed_log_records_without_hasura_row` and `hasura_rows_without_signed_log`. |
| `braid` | The `status` of the Braid verifier, the `board_name`, `tally_batches` and `board_ballot_batches`. |

A count is `null` when its step did not run.

## Files of export-ballot-audit (postgres-export)

| File | Content | Personal data |
| --- | --- | --- |
| `scope.json` | An exact copy of the scope file that you gave. | No |
| `cast-votes.json` | All stored ballots of the area, with all statuses and all revotes, oldest first. One object for each row of the `cast_vote` table. | Yes |
| `cast-vote-history.json` | All stored ballots of the election in all areas, oldest first. Same format as `cast-votes.json`. | Yes |
| `eligible-voters.json` | The frozen list of voters that can vote in the area. See [eligible-voters.json](#eligible-votersjson). | Yes |
| `eligibility-reasons.json` | For each user of the area, if the user can vote and why. See [eligibility-reasons.json](#eligibility-reasonsjson). | Yes |
| `keycloak-source.json` | The Keycloak data that the list of voters comes from: the realm, the users of the area with their attributes, and the delegations. | Yes |
| `board-messages.json` | A list of messages: the configuration, the public key and the `Ballots` statements of the scope. | No |
| `board-source.json` | The board index row (`index`), one object for each message with its ID, sender, kind, batch, mix number and digest (`rows`), and the count check of the board (`index_count`). | No |
| `saved-mapping.json` | The saved records of the tally: `election_event`, `election`, `tally_session`, `tally_session_contest`, `tally_session_executions` and `keys_ceremony`. | No |
| `acquisition.json` | The record of the export. See [acquisition.json](#acquisitionjson). | No |

### Fields of a stored ballot

`cast-votes.json` and `cast-vote-history.json` contain all columns of the `cast_vote` table. The
audit uses these fields:

| Field | Content |
| --- | --- |
| `id` | The ID of the stored ballot. The reports call it `cast_vote_id`. |
| `tenant_id`, `election_event_id`, `election_id`, `area_id` | The place of the ballot. |
| `voter_id_string` | The voter ID. |
| `ballot_id` | The receipt: the first half of the SHA-512 ballot hash. |
| `content` | The encrypted ballot, as text. |
| `status` | The status of the row, for example `valid` or `discarded`. |
| `created_at` | The time that the platform stored the ballot. |
| `annotations` | Other data of the row, for example `voting_channel`. |

### eligible-voters.json

| Field | Content |
| --- | --- |
| `format_version` | `1` |
| `scope_sha512` | The pin of the scope file. |
| `voters[].voter_id` | The voter ID. |
| `voters[].multiplicity` | The number of votes that the ballot of this voter counts for: `1`, the vote weight, or `1` plus the number of delegations. |

### eligibility-reasons.json

| Field | Content |
| --- | --- |
| `source_users` | The number of users of the area. |
| `eligible_users` | The number of voters in `eligible-voters.json`. |
| `reason_counts` | The number of users for each reason. A user can have more than one reason. |
| `decisions[]` | One object for each user: `voter_id`, `eligible`, `reasons` and `multiplicity`. A reason is `eligible`, `not_enabled`, `election_not_authorized` or `service_account`. |
| `delegation_diagnostics` | Only for the policy `delegated`: delegations from other realms and from users that are not enabled. |

### acquisition.json

| Field | Content |
| --- | --- |
| `source_revision` | The source code revision of the command. |
| `status` | `postgres_exports_complete_electoral_log_required`: the next step is the electoral log export. |
| `files` | The size (`bytes`) and the digest (`sha512`) of each file of the directory. |
| `snapshots` | For each copy (`hasura`, `keycloak`, `b3`): the database, the restore reference from `connections.json`, the time of the read, and the read-only settings. |
| `counts` | The number of stored ballots in the area and in the election, of eligible voters and of board messages. |
| `board_index_count` | The count of messages that the board index stores, compared with the count that the export found. A difference is not a finding. |
| `cast_vote_history_scope` | The filter of `cast-vote-history.json`: all areas, all statuses, all revotes. |
| `statement_timeout_seconds` | The time limit of each query. |
| `limitations` | Text. What the export does not prove. |

:::note
The digests in `acquisition.json` come from the same command that made the files. They show that
the files did not change after the export. They do not replace the pins in the audit notes.
:::

## Files of export-electoral-log (immudb-export)

| File | Content | Personal data |
| --- | --- | --- |
| `electoral-log.json` | A list of messages: the signed `CastVote` records of the election event. | Yes |
| `electoral-log-provenance.json` | The record of the export. | No |

`electoral-log-provenance.json` has these fields:

| Field | Content |
| --- | --- |
| `source` | `local_restored_immudb` |
| `database` | The database that the command read. |
| `read_only_transaction` | `true` |
| `filter` | The statement kind (`CastVote`), the election event and, if you gave it, the election. |
| `message_count` | The number of messages in `electoral-log.json`. |
| `other_event_messages_excluded` | The number of records of other election events that the command left out. |
| `electoral_log_sha512` | The digest of `electoral-log.json`. |
| `limits` | Text. What the export does not prove. |

## Files of prepare-ballot-audit (prepared-audit)

| File | Content | Personal data |
| --- | --- | --- |
| `manifest.json` | The scope, the batches and the selected ballots. See [manifest.json](#manifestjson). | Yes |
| `configuration.borsh` | The signed configuration of the bulletin board, in binary form. | No |
| `cast-votes.json` | The selected ballots. Each object has `voter_id`, `voting_channel`, `cast_vote_id`, `ballot_id` and `content`. | Yes |
| `messages.json` | A list of messages: the `Ballots` statements of the scope. | No |
| `electoral-log.json` | A list of messages: the signed records of the selected ballots only. | Yes |
| `anchors.json` | The digests of `manifest.json` and `configuration.borsh`, calculated by the same command. | No |
| `provenance.json` | The record of the preparation. See [provenance.json](#provenancejson). | No |
| `selection.json` | The decision for each stored ballot of the area. See [selection.json](#selectionjson). | Yes |

:::caution CAUTION
Do not use the digests in `anchors.json` as pins. The same command wrote them. A changed bundle
can contain changed anchors.
:::

### manifest.json

| Field | Content |
| --- | --- |
| `scope` | The scope, the same as `scope.scope` in `scope.json`. |
| `batches` | The batches of the scope. |
| `selected[].cast_vote_id` | The ID of the stored ballot. |
| `selected[].ballot_id` | The receipt. |
| `selected[].content_sha512` | The digest of the encrypted ballot. |
| `selected[].electoral_log_message_sha512` | The digest of the signed record of this ballot in the electoral log. |
| `selected[].multiplicity` | The number of votes that this ballot counts for. |

### provenance.json

| Field | Content |
| --- | --- |
| `source_sha512` | The pins that you gave for each input file. |
| `anchors` | The same digests as `anchors.json`. |
| `selection_accounting` | The counts of the selection. The same as `accounting` in `selection.json`. |
| `tally_annotations` | The comparison with the saved tally. `status` is `checked` with the values, `not_supplied` if you did not give `--saved-mapping`, or `annotations_absent` if the saved tally has no annotations. |
| `cast_vote_history` | The history checks. `status` is `checked` with the counts, or `skipped` if you gave `--without-cast-vote-history`. |
| `board_public_key` | The check of the public key hash of the scope against the bulletin board. `status` is `matched` with the number of signed statements, or `not_on_board`. |
| `collapsed_identical_electoral_log_records` | The number of identical signed records that the command counted once. |
| `checked_cast_votes`, `checked_batches`, `checked_electoral_log_messages` | The numbers of checked items. |
| `legacy_online_channel_inferences` | The number of signed records in the earlier format, without a voting channel. The command used the online channel for them. |
| `limitations` | Text. What the preparation does not prove. |

### selection.json

| Field | Content |
| --- | --- |
| `scope_sha512` | The pin of the scope file. |
| `accounting` | The counts: rows for each reason, eligible voters without a valid vote, selected input positions, exclusions and votes by channel. |
| `rows[].cast_vote_id` | The ID of the stored ballot. |
| `rows[].ballot_id` | The receipt, as stored. The command checked only the receipts of selected ballots. |
| `rows[].reason` | `selected`, `superseded_revote`, `discarded`, `no_eligible_voter` or `outside_scope`. See [Read the Results](03-read-the-results.md#reasons-for-an-excluded-ballot). |
| `rows[].latest_scoped_cast_vote_id` | For a revote: the ID of the retained cast row for the same voter. |
| `rows[].resolved_multiplicity` | The number of votes of the voter, if the voter is in the frozen list. |
| `rows[].receipt_verification` | `selected_audit_verified` or `not_checked`. |
| `limitation` | Text. What the selection does not prove. |

## Reports

`ballot-audit` and `audit-cast-history` write the report to the standard output. The procedures
keep it in a file.

### audit-report.json and lookup.json (ballot-audit)

| Field | Content |
| --- | --- |
| `scope` | The scope of the bundle. |
| `checked_cast_votes`, `checked_batches`, `checked_electoral_log_messages` | The numbers of checked items. |
| `legacy_online_channel_inferences` | The number of signed records in the earlier format, without a voting channel. |
| `receipts[]` | For each selected ballot: `cast_vote_id`, `ballot_id` and `locations`, the batch and the position of the ballot in the batch. |
| `lookup` | Only with `--ballot-id`. The `outcome` (`selected`, `excluded`, `missing` or `ambiguous`) and its details. |
| `duplicate_selected_receipts[]` | Receipts that more than one selected ballot has. A value here is a finding. |
| `privacy_warnings[]` | Weighted batches holding one to four ciphertexts: `batch`, `ciphertexts` and `minimum_for_anonymity`. |
| `assurance` | Text. What the audit proves. |

:::danger WARNING
The `receipts` list connects a receipt to a batch position. Do not publish it. In a small batch,
the position can show how a person voted.
:::

### history-report.json (audit-cast-history)

| Field | Content |
| --- | --- |
| `outcome` | `consistent`, `contradicted`, `incomplete` or `ambiguous`. |
| `checked_hasura_rows` | The number of stored ballots that the command checked. |
| `checked_signed_cast_vote_log_records` | The number of signed records that the command checked. |
| `ignored_other_log_records` | The number of records of other elections. |
| `signed_log_records_without_hasura_row` | Signed records without a stored ballot. A value other than `0` is a finding. |
| `hasura_rows_without_signed_log` | Stored ballots without a signed record. This is incomplete evidence. |
| `ambiguous_row_attribution_groups`, `ambiguous_log_occurrence_groups` | The numbers of ambiguous groups. |
| `report_filtered`, `requested_receipt_found` | Only with `--ballot-id`: the report shows one receipt, and if the command found it. |
| `receipts[]` | One group for each receipt: `ballot_id`, `cast_vote_ids`, the counts of signed records, and the `classification`. See [Check the Cast-Vote History](07-run-each-step/07-history.md#2-read-the-groups). |
| `source_sha512` | The pins of the input files. |
| `assurance` | Text. What the check proves. |
