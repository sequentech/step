---
id: developers_velvet
title: Developers Velvet
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->




This is a placeholder page for the section: Velvet.

Content will be added here soon.

### Auditable cast ballots

Windmill retains an audit snapshot when extracting ballots for a new tally session.
The tally archive contains `auditable-ballots/election__<id>/contest__<id>/area__<id>/`
for single-contest encryption, or `auditable-ballots/election__<id>/area__<id>/`
for multiple-contest encryption. Both original and renamed archives include it.
Each directory contains `summary.json` (count and `ballots_available`) and, when
available, `encrypted-ballots.jsonl` with one unsigned encrypted ballot per line.
Voter IDs, voter signing keys and voter signatures are not exported in this folder.

The count adds ballots marked `discarded` (including voter-disable releases) to
existing ballots without an enabled, authorized voter. Extraction keeps the latest
valid revote when present, otherwise the latest discarded ballot for each voter in
the election/area. Discarded ballots never enter the mix, even after re-enabling the
voter. Disabled voters remain outside the eligible census. These audit counts do
not replace the existing invalid/incorrectly encoded vote classifications or
Velvet's decoded-ballot outputs.

Recounts reuse the stored snapshot. Sessions extracted before audit snapshots were
introduced retain their recorded count, with `ballots_available: false` when the
count is nonzero and no snapshot exists. Start a new tally session to apply the new
extraction rules; rerunning an old session does not reconstruct a historical
snapshot from the current voter database. Event imports remap snapshot references
when S3 files are included; imports without them retain counts and report the
snapshot as unavailable.
