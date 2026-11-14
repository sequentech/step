---
id: import_file_formats
title: Import File Formats
description: "The formats of the files that you can import in the admin portal: what each file contains, how to make it, and examples."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This section gives the format of each file that the admin portal can import, with one page for
each file. Most files are files that you export from the admin portal. Some files, for example
the voter file, you make yourself.

## Summary

| File | Where you import it | Type | You make it yourself? |
| --- | --- | --- | --- |
| [Election event](01-election-event.md) and its [elections, contests and candidates](02-election-contest-candidate.md) | **Election Events** > **+** > **Import Election Event** | `.json`, `.zip`, `.ezip` | No. Use an export. |
| [Voters](03-voters.md) | Election event > **Voters** > **Import** | `.csv`, `.tsv` | Yes |
| [Areas](04-areas.md) | Election event > **Areas** > **Import** | `.csv` | Yes |
| [Applications](05-applications.md) | Election event > **Approvals** > **Import** | `.csv` | No. Use an export. |
| [Templates](06-templates.md) | **Templates** > **Import** | `.csv` | No. Use an export. |
| [Tenant backup](07-tenant-backup.md) | **Settings** > **Backup / Restore** | `.zip` | No. Use an export. |
| [Tally sheets](08-tally-sheets.md) | Election event > **Tally sheet imports** | `.csv` | Yes |
| [Private key backup](09-private-key-backup.md) | Keys ceremony and tally ceremony | `.txt` | No. Use the downloaded file. |
| [Election presentation](10-election-presentation.md) | Election > **Data** > advanced settings | `.json` | Yes, with care |
| [CA certificates](11-ca-certificates.md) | Election event > **Certificates** | PEM text | No. Get it from the certificate authority. |

Each import panel has the field **Integrity Check (SHA-256)**. Type the SHA-256 hash of the file
there. The import then refuses a file that somebody changed. In this version the election event
import does not check the hash: see [Election Event File](01-election-event.md). To get the hash
of a file:

```bash
shasum -a 256 <file>
```

On Windows, use `certutil -hashfile <file> SHA256`.

## Other imports

- **Import candidates** on the **Data** tab reads the fixed file layout of a specific external
  integration. It is not a general-purpose import. To add candidates, use the election event tree
  or import an election event.
- **Voter list reconciliation** reads the fixed file layout of a specific external voter-list
  system. See [Voters](../02-election-event/05-election_management_election-event_voters.md).
- The signing file of a transmission and the images and documents that you add to candidates and
  support materials are not in a Step format.
