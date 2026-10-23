---
id: run_each_step
title: "Appendix: Run Each Step Separately"
sidebar_position: 0
description: "Run the commands of audit-tally one at a time, for one scope, to repeat or examine one step of the audit."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Appendix: Run Each Step Separately

[Run the Audit](../02-procedures/03-run-the-audit.md) runs all commands of the audit with
`audit-tally`. This appendix runs the same commands one at a time, for one scope: one election,
one area and one contest of the tally.

Use this appendix to do these tasks:

- Build the tools and restore the copies without the audit kit.
- Repeat a step that did not pass, after you correct its cause.
- Examine the files of one step.
- Look up the receipt of a voter. See [Audit the Bundle](05-audit.md#2-look-up-a-receipt).

Do the procedures in this order. **Check the Cast-Vote History** is optional when you do only one
scope.

| Step | Procedure | Result |
| --- | --- | --- |
| | [Without the Audit Kit](01-without-kit.md) | Only if you do not use the audit kit: the tools and the copies. |
| 1 | [Get the Scope and the Pins](02-scope.md) | A pinned `scope.json` file. |
| 2 | [Export the Evidence](03-export.md) | The evidence files and their digests. |
| 3 | [Prepare the Audit Bundle](04-prepare.md) | An audit bundle. |
| 4 | [Audit the Bundle](05-audit.md) | An audit report. |
| 5 | [Verify the Bulletin Board](06-braid.md) | A Braid result for each batch. |
| 6 | [Check the Cast-Vote History](07-history.md) | A history report. |

## Use the procedures with the audit kit

The procedures give the commands without the audit kit. With the kit, the commands run in the
tools container. Change them as this table shows:

| In the procedure | With the audit kit |
| --- | --- |
| `cd ~/audit`, then a command such as `export-ballot-audit ...` | `audit-env exec --dir ~/audit -- export-ballot-audit ...`. The command runs in `~/audit/results`. Write `scope.json` there. |
| `> audit-report.json` and other redirections | The shell of your computer writes the file. Do `cd ~/audit/results` first. |
| `connections.json` | `/audit/private/connections.json` |
| `docker exec -it audit-hasura-db psql -U audit_reader -d hasura_copy` | `audit-env sql --dir ~/audit hasura`. Use `keycloak` or `b3` for the other copies. |
| `--username "$IMMUDB_READ_ONLY_USER"` and the ImmuDB password | `--username ballot_audit_reader`. `audit-env exec` sets the password. |
| `--b3-blob-root <path>` | `--b3-blob-root /audit/b3-blobs` |
| `verify --server-url http://127.0.0.1:50051 ...` | `audit-env exec --dir ~/audit -- verify --server-url http://127.0.0.1:50051 ...` |

:::note
`audit-tally` writes the files of each scope in `results/tally/scopes/<NNN>/`. To examine one
step of a run, use the command line of the step in `results/tally/audit-record.json`. To repeat
it, use a new output directory.
:::

After the last procedure, do [Report and Remove the Copies](../02-procedures/04-close.md).
