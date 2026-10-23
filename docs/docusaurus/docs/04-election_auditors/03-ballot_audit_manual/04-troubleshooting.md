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
