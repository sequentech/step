---
id: import_file_formats
title: Import File Formats
sidebar_position: 12
description: "The formats of the files that you can import in the admin portal: what each file contains, how to make it, and examples."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This page gives the format of each file that the admin portal can import. Most files are files
that you export from the admin portal. Some files, for example the voter file, you make yourself.

## Summary

| File | Where you import it | Type | You make it yourself? |
| --- | --- | --- | --- |
| [Election event](#election-event) | **Election Events** > **+** > **Import Election Event** | `.json`, `.zip`, `.ezip` | No. Use an export. |
| [Voters](#voters) | Election event > **Voters** > **Import** | `.csv`, `.tsv` | Yes |
| [Areas](#areas) | Election event > **Areas** > **Import** | `.csv` | Yes |
| [Applications](#applications) | Election event > **Approvals** > **Import** | `.csv` | No. Use an export. |
| [Templates](#templates) | **Templates** > **Import** | `.csv` | No. Use an export. |
| [Tenant backup](#tenant-backup) | **Settings** > **Backup / Restore** | `.zip` | No. Use an export. |
| [Tally sheets](#tally-sheets) | Election event > **Tally sheet imports** | `.csv` | Yes |
| [Private key backup](#private-key-backup) | Keys ceremony and tally ceremony | `.txt` | No. Use the downloaded file. |
| [Election presentation](#election-presentation) | Election > **Data** > advanced settings | `.json` | Yes, with care |
| [CA certificates](#ca-certificates) | Election event > **Certificates** | PEM text | No. Get it from the certificate authority. |

Each import panel has the field **Integrity Check (SHA-256)**. Type the SHA-256 hash of the file
there. The admin portal then refuses a file that somebody changed. To get the hash of a file:

```bash
shasum -a 256 <file>
```

On Windows, use `certutil -hashfile <file> SHA256`.

## Election event

You import an election event to make a copy of an earlier election event, or to move it to a
different tenant. Make the file with **Export** on the election event. Do not change the file.

| File type | Content |
| --- | --- |
| `.json` | The configuration only: the election event, its elections, contests, candidates, areas and the links between areas and contests. |
| `.zip` | The JSON file and, if you selected them in the export, the voters, reports, scheduled events, publications, documents, images, logs, keys and tally data. |
| `.ezip` | A `.zip` file that is encrypted with a password. Type the password in **Decryption Password**. |

The export gives the file the name `election-event-<ID>-export-sha256-<hash>.zip` (or `.ezip`).
The name contains the SHA-256 hash of the file.

The import:

- Adds the election event to the tenant that you are signed in to. The tenant ID in the file is
  not used.
- Gives the election event and all its parts new IDs.
- Resets the status, the voting periods and the statistics. The election event has no election
  key. Do a new keys ceremony.
:::caution CAUTION: hash of the election event file
In this version the admin portal does not send the hash of an election event file to the server,
so the import does not check it. Compare the hash yourself before you import: the hash that
`shasum` gives must be the same as the hash in the file name. A `.zip` or `.ezip` export can
contain voter data and keys. Keep it in a protected location.
:::

## Voters

A text file with one row for each voter. The first row is the header. The separator is a comma.
For a tab separator, give the file the extension `.tsv`. The order of the columns is not
important.

```csv
username,email,first_name,last_name,area_name,enabled,authorized-election-ids
voter001,voter001@example.com,Ana,Lopez,North,TRUE,
voter002,voter002@example.com,Ben,Smith,South,TRUE,election-1|election-2
```

| Column | Content |
| --- | --- |
| `username` | The user name. If there is no `username` column, the import makes a random user name. |
| `email` | The email address. |
| `first_name`, `last_name` | The name of the voter. |
| `area_name` | The name of the area, as on the **Areas** tab. |
| `enabled` | `TRUE` or `FALSE`. If empty, `TRUE`. |
| `password` | Optional. The password. The import stores only a hash of it. |
| `hashed_password`, `password_salt`, `num_of_iterations` | Optional. A password that is already hashed with PBKDF2-SHA256, and its salt, in Base64. Use them in place of `password`. |
| `vote-weight` | Optional. A whole number from 1 to 100 000. Only for weighted voting. |
| `authorized-election-ids` | Optional. The External IDs of the elections that the voter can vote in. |
| Other columns | Each other column becomes an attribute of the voter with the same name, for example `sequent.read-only.mobile-number` or `dateOfBirth`. |

Column names can contain only letters, digits, `.`, `_` and `-`. To give more than one value to
an attribute, separate the values with `|`. For the steps, see [Add the Voters](../03-procedures/03-voters.md).

## Areas

A CSV file without a header row. Each row has six columns:

```csv
North,,,North District,0,no_early_voting
South,,,South District,0,allow_early_voting
```

| Column | Content |
| --- | --- |
| 1 | The **Name** of the area. |
| 2, 3 | Not used. Leave them empty. |
| 4 | The **Description** of the area. |
| 5 | `0` to import the row. Rows with another value are not imported. |
| 6 | `allow_early_voting` or `no_early_voting`. If empty, `no_early_voting`. |

The import links each area to all the contests of the election event. For the steps, see
[Create the Election Event](../03-procedures/02-event.md#import-areas-alternative).

## Applications

Make the file with **Export** on the **Approvals** tab. The file has a header row and twelve
columns:

```csv
id,created_at,updated_at,tenant_id,election_event_id,area_id,applicant_id,applicant_data,labels,annotations,verification_type,status
```

The import uses the tenant ID and the election event ID of each row. Import the file only into
the election event where you exported it. See
[Approvals](02-election-event/14-election_management_election-event_approvals.md#import-file).

## Templates

Make the file with **Export** on the **Templates** page. The file has a header row and ten columns:

```csv
alias,tenant_id,template,created_by,labels,annotations,created_at,updated_at,communication_method,type
```

The import uses the tenant ID of each row. Import the file only into the tenant where you exported
it. See [Templates](02-election-event/13-02-election_management_election-event_templates.md#exporting-and-importing-templates).

## Tenant backup

Make the file with **Backup** in **Settings** > **Backup / Restore**. The file has the name
`tenant-config-<tenant ID>-export.zip` and contains:

| File in the ZIP | Content |
| --- | --- |
| `tenant_configuration-<tenant ID>.csv` | The settings of the tenant. |
| `keycloak_admin-<tenant ID>.json` | The login configuration of the tenant. The restore uses only its display name and its texts. |
| `export_permissions-<tenant ID>.csv` | The roles and their permissions, with the header `role,permissions`. The permissions of a role are separated with `\|`. |

The restore changes the tenant that you are signed in to. It does not use the tenant ID in the
file. In this version you must select all three restore options. See
[Backup & Restore](user-manual/settings/settings_backup-restore.md).

## Tally sheets

The results of paper, postal or in-person voting for each area and contest. Select **Canonical CSV** as the
source format. The file has a header row. The order of the columns is not important.

```csv
channel,area_name,contest_external_id,field,candidate_external_id,value
PAPER,North,contest-1,census,,1200
PAPER,North,contest-1,total_votes,,815
PAPER,North,contest-1,total_valid_votes,,790
PAPER,North,contest-1,total_blank_votes,,10
PAPER,North,contest-1,implicit_invalid,,9
PAPER,North,contest-1,explicit_invalid,,16
PAPER,North,contest-1,candidate_votes,candidate-1,480
PAPER,North,contest-1,candidate_votes,candidate-2,300
```

| Column | Content |
| --- | --- |
| `channel` | `PAPER`, `POSTAL` or `IN_PERSON`. It must be the channel that you select in the import panel. |
| `area_name` | The name of the area. The area must be linked to the contest. |
| `contest_external_id` | The **External ID** of the contest. |
| `field` | `census`, `total_votes`, `total_valid_votes`, `total_blank_votes`, `implicit_invalid`, `explicit_invalid`, `blank_ballots` or `candidate_votes`. |
| `candidate_external_id` | Only for `candidate_votes`: the **External ID** of the candidate. Leave it empty for the other fields. |
| `value` | A whole number, 0 or more. |

Each area and contest needs one row for each field except `blank_ballots`, which is optional, and
one `candidate_votes` row for each candidate. Each candidate of the contest must have a unique
External ID, and the External ID of each contest must be unique in the election event. The admin portal shows the errors before you import. The other source format is the
XML results format of an external system. See
[Tally Sheet Imports](02-election-event/08-03-election_management_election-event_tally-sheet-imports.md).

## Private key backup

In the keys ceremony, each trustee downloads the file
`encrypted_private_key_trustee_<user name>_<election event name>.txt`. The trustee uploads the
same file to check it in the keys ceremony, and again in the tally ceremony. Do not change the
file, and do not open it in a text editor that can save changes. The admin portal compares it with
the encrypted key that it keeps. If one character is different, it shows "Invalid Encrypted
Private Key Backup".

## Election presentation

An advanced option. In the **Data** tab of an election, you can upload a `.json` file with
presentation settings. Each top-level key in the file replaces the same key in the presentation of
the election. The other keys do not change. Upload only a file that you got from Sequent support or
from the presentation of another election.

## CA certificates

The certificates of the certificate authorities that can sign the certificates of voters. Upload a
text file in PEM format. The file can contain one or more certificates, each between
`-----BEGIN CERTIFICATE-----` and `-----END CERTIFICATE-----`. The admin portal skips expired
certificates and certificates that you already imported, and shows how many it added. The upload
works only when the voter certificate policy of the election event is on. See
[Certificates](02-election-event/15-election_management_election-event_certificates.md).

## Other imports

- **Import candidates** on the **Data** tab reads the fixed file layout of a specific external
  integration. It is not a general-purpose import. To add candidates, use the election event tree
  or import an election event.
- **Voter list reconciliation** reads the fixed file layout of a specific external voter-list
  system. See [Voters](02-election-event/05-election_management_election-event_voters.md).
- The signing file of a transmission and the images and documents that you add to candidates and
  support materials are not in a Step format.
