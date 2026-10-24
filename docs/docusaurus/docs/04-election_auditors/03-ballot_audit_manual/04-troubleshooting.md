---
title: Troubleshooting
sidebar_position: 4
description: "The messages of the ballot audit commands, their exit status, their cause and the action to do."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Troubleshooting

This page lists the messages of the ballot audit commands. The exit status is in brackets.

## Messages of the audit image

The audit image shows its own messages after `ballot-audit:`. It stops with exit status `1` after
each of these messages, before the audit. This exit status is not a result of the audit. When the
audit runs, the exit status is the exit status of `audit-tally`.

| Message | Cause | Action |
| --- | --- | --- |
| `backup digest mismatch: backups/...; do not restore it` | The backup is not the backup that the system operator described. | Stop. Do not use the backup. Tell the election authority. |
| `missing backup: backups/...` | A backup is not in the directory, or it has another name. | Put the backup in the backups directory with the name of [The audit image](05-command-reference.md#the-audit-image). |
| `no digest for ... in backup-digests.txt` | The digests file has no line for a backup. | Add the digest from the system operator. |
| `no backup-digests.txt in the backups directory` | The digests file is missing. | Write it. See [Prepare the Backups](02-procedures/02-restore.md). |
| `no backups; mount the backups directory at /backups` | The command has no `-v <backups>:/backups:ro`. | Add it. |
| `no output; mount an empty output directory at /output` | The command has no `-v <output>:/output`. | Add it. |
| `the output directory is not empty; use a new one` | The output directory has files of an earlier run. | Use a new, empty directory. |
| `WARNING: pg_restore reported ... error(s) for ...` | `pg_restore` could not restore part of a backup. The command continues. | Read the log in `output/restore`. Write each error in the audit notes. |
| `the ImmuDB archive has no systemdb directory` | The ImmuDB archive is not an ImmuDB data directory. | Ask the system operator for the correct archive. |
| `the ImmuDB copy has no databases` | The ImmuDB data directory is empty. | Ask the system operator for the correct archive. |
| `ImmuDB did not start`, `the B3 server did not start` | A copy did not start. The command shows the end of its log. | Read the logs in `output/restore`. |
| `unknown option: ...`, `... needs a value` | An option of the command is wrong. | Correct the command. See [The audit image](05-command-reference.md#the-audit-image). |

## Messages of audit-tally

`audit-tally` shows its own messages after `audit-tally:`. When a step does not pass, the `message`
of the step in `audit-record.json` gives the message of that command. Find that message in the
other tables of this page.

| Message | Cause | Action |
| --- | --- | --- |
| `the following required arguments were not provided` [2] | An option is missing. You must give `--immudb-server-url` with `--immudb-username`, or `--electoral-log-export`. | Correct the command. See [Command Reference](05-command-reference.md#audit-tally). |
| `invalid tally session ID`, `tally session ID must use canonical lowercase spelling` [4] | The tally ID is not a UUID in lowercase. | Get the tally ID again from the election authority. Use lowercase. |
| `public key SHA-512 pin must be 128 lowercase hexadecimal characters` [4] | The public key hash is not valid. | Get the hash again from the election authority. |
| `invalid electoral log system public key` [4] | The key is not a DER/SPKI key in standard base64. | Get the key again from the election authority. |
| `cannot locate the ballot audit commands` [4] | The command cannot find its own directory. | Build the audit image again. Without the audit image, run `audit-tally` from `target/release`, or give its full path. |
| `expected exactly one tally session row, found none` [3] | The Hasura copy has no tally with this ID. | Check the tally ID. Make sure that the backup is from after the tally. |
| `expected exactly one election event row, found none` [3] | The Hasura copy has no election event for the tally. | Make sure that the Hasura backup is complete. |
| `saved tally has no contest rows` [3] | The tally has no areas or contests yet. | Get a backup from after the start of the tally. |
| `saved election event lacks its board name`, `saved ... lacks ...` [3] | A saved record of the tally has no value for a field that the audit needs. | Make sure that the Hasura backup is complete. |
| `saved session has no configuration column`, `saved tally contest has an invalid batch`, `saved tally contest has an invalid contest ID` [4] | The saved tally is from an unsupported release. | Check the version of the data. See [Versions](01-before-you-start.md#versions). |
| `invalid B3 board SQL identifier` [4] | The board name is not a valid name. | Write in the audit report that the tools cannot audit this board. |
| `cannot read B3 board batches`, `B3 board has an invalid batch` [4] | The B3 copy has no table for the board, or the table is not readable. | Check the restore of the B3 copy and the access of the read-only account. |
| `output directory already exists` [4] | The output directory exists. | Use a new output directory. |

### Messages in the audit record

| `message` of a step | Cause | Action |
| --- | --- | --- |
| `cannot start <program>` | The command is not next to `audit-tally`, or `--braid-verify` is wrong. | Build the audit image again. Without the audit image, build all tools into `target/release`. |
| `no report on the standard output`, `cannot save the report` | The command did not write a report, or the report could not be saved. | Read the rest of the message. Check the free disk space. |
| `existing electoral log export` with the status `invalid_input` | The directory of `--electoral-log-export` has no `electoral-log.json`. | Give the directory that `export-electoral-log` wrote. |
| `the export did not succeed`, `the preparation did not succeed` | An earlier step of the scope did not pass. | Correct the cause of the earlier step. |
| `no electoral log` | The electoral log export did not pass. | Correct the cause of the `export-electoral-log` step. |
| `no successful export` | No scope of the election was exported. | Correct the cause of the `export-ballot-audit` steps. |
| `no B3 server address` | You did not give `--b3-server-url`. | Start the B3 server and run the audit again with `--b3-server-url`. |
| `cannot write scope.json` | The command could not write the scope file. | Check the free disk space and the access to the output directory. |

## Input and connection messages

| Message | Cause | Action |
| --- | --- | --- |
| `... SHA-512 pin must be 128 lowercase hexadecimal characters` [4] | The pin has uppercase characters, is too short or has spaces. | Calculate it again with `shasum -a 512`. Use lowercase. |
| `remote database connections are refused`, `use a loopback IP, not a hostname` [4] | The connection uses a host name or a remote address. | Use `127.0.0.1` or a Unix socket. |
| `... does not support TLS-required connections` [4] | The connection has `sslmode=require`. | Remove the setting. The copy is on the loopback address. |
| `connections file must not grant group or other access` [4] | Other users can read `connections.json`. | Run `chmod 600 connections.json`. |
| `output directory already exists`, `cannot create new private output directory` [4] | The output directory exists, or its parent does not exist. | Use a new output directory in a directory that exists. |
| `non-default keys-ceremony boards require separately authenticated ...` [4] | The election uses a key ceremony that the tools do not support. | Write in the audit report that the tools cannot audit this election. |

## Scope messages

| Message | Cause | Action |
| --- | --- | --- |
| `manifest digest mismatch`, `configuration digest mismatch`, `... source digest mismatch`, `scope digest mismatch` [1] | A file is not the file that the pin describes. | Get the file again. Do not calculate a new pin from the changed file. |
| `... differs from pinned scope` [1] | The scope names another tally, contest, area, base batch, board or policy than the saved tally. | Check each value of `scope.json` against the tally. |
| `saved batch set differs from pinned scope` [1] | The list of batches is not the list of the saved tally. | Make the list again from the base batch and the weight bit mask. |
| `combined weighted and delegated voting is unsupported`, `unsupported saved tally configuration` [4] | The tally uses a setup that the tools do not support. | Write in the audit report that the tools cannot audit this tally. |
| `weighted saved mapping lacks a weight bit mask` [3] | The weighted tally has no saved weight bit mask. | Get a backup from after the end of the tally. |

## Evidence messages

| Message | Cause | Action |
| --- | --- | --- |
| `scoped electoral-log signature verification failed` [1] | A signed record is changed, or the system public key in the scope is wrong. | Check the key with the election authority. If it is correct, report the finding. |
| `pinned public key hash differs from the board PublicKey statement` [1] | The public key hash in the scope is wrong, or the key on the board was replaced. | Check the hash with the election authority. If it is correct, report the finding. |
| `a selected voter has a signed cast vote that no stored cast vote holds ...` [1] | A stored ballot was removed or replaced. | Keep the evidence and report the finding. |
| `a selected voter has a later signed revote held by a valid scoped cast vote ...` [1] | The stored vote order disagrees with the signed log. | If the times are seconds apart, ask for a check of the server clocks. Then report it. |
| `cast-vote history does not hold a selected cast vote ...` [1] | The history and the ballots come from different moments, or one was changed. | Export both again in one export. |
| `... ballot receipt(s) are shared by several selected cast votes ...` [1] | One encrypted ballot was counted for more than one voter. | Keep the evidence and report the finding. |
| `saved eligible voter count differs ...`, `saved cast ballot count differs ...`, `saved ballots without voter differ ...` [1] | The saved tally disagrees with the selection. | Ask about changes after the tally. See [Read the Results](03-read-the-results.md#contradiction-or-a-change-after-the-tally). |
| `tally input multiset mismatch ...` [1] | The encrypted ballots on the bulletin board are not the selected ballots. | Keep the evidence and report the finding. |
| `selected cast vote has no matching signed electoral-log evidence` [3] | The signed record of a counted ballot is missing. | Report it as incomplete. |
| `selected cast vote has ambiguous signed electoral-log evidence` [3] | Different signed records describe the same ballot. | Report it as incomplete. |
| `scoped in-progress cast vote blocks preparation` [3] | A vote was in progress at the backup. | Get a backup from after the end of the voting period. |
| `restored B3 board has no configuration or scoped ballots`, `missing B3 batch` [3] | The B3 copy or the blob directory does not have a batch. | Check the restore and `--b3-blob-root`. |
| `required checks failed or were incomplete` (Braid) | A check of the bulletin board failed, a batch has no decrypted results, or `--batches` is not the list of the board. | Read the summary. Give all batches of the board. |
