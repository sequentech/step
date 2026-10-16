---
id: import_format_ca_certificates
title: CA Certificate File
sidebar_position: 11
description: "The format of the CA certificate file: the PEM blocks, the rules, the fields the portal shows for each certificate and the results of the import."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The CA certificate file is a text file in the PEM format. It holds one or more certificates of
certificate authorities (CAs). The platform uses the imported CAs to validate voter certificates.
You import the file in an election event, on the **Certificates** tab, with **Import
Certificates** or **Import**. You do not make the file yourself. You get it from the CA or from the
source of the certificates.

## Before you import

The **Certificates** tab is shown only when the voter certificate policy is enabled.

1. Open the **Data** tab of the election event.
2. Set **Voter Digital Certificate Policy** to **Enabled**.
3. Save the election event.

The import is refused while the policy is **Disabled**. The user needs the permission **Edit
certificate authorities**.

## What the file is

| Property | Value |
| --- | --- |
| Type | Plain text, PEM format. |
| Content | One or more certificate blocks. Other text is ignored. |
| Block start | The line `-----BEGIN CERTIFICATE-----`. Spaces before or after the line are ignored. |
| Block end | The line `-----END CERTIFICATE-----`. Spaces before or after the line are ignored. |
| Block body | The Base64 text of one X.509 certificate, as in a PEM file. |
| One block for | One certificate. |
| Order | Any order. Each block is imported on its own. |

Example of the layout. The Base64 lines are placeholders. This file does not import:

```text
Text before the first certificate is ignored.
-----BEGIN CERTIFICATE-----
MIIB...
...more Base64 lines...
-----END CERTIFICATE-----
-----BEGIN CERTIFICATE-----
MIIB...
-----END CERTIFICATE-----
```

## Rules

| Rule | Value |
| --- | --- |
| Block without an end line | The block is ignored. The portal shows no message for it. |
| Text inside a block | The server reads the lines of the block as Base64. Do not add other text inside a block. A line of other text gives an error for that certificate. |
| Invalid certificate | An error for this certificate only. The other certificates are still imported. |
| Expired certificate | Not imported. The result is an error. |
| Duplicate certificate | Skipped. A certificate is a duplicate when its SHA256 fingerprint is already in the election event. |
| Scope | The certificates belong to one election event. The same certificate can be imported in another election event. |
| No block in the file | The import is refused. |
| Policy disabled | The import is refused. No certificate is imported. |
| Delete | A deleted certificate is removed from the election event. The delete is written to the electoral log. |

## Fields shown in the portal

The **Certificates** tab shows one row for each certificate. The table lists the columns of the
tab.

| Column | Source | Meaning |
| --- | --- | --- |
| Common Name | The common name (CN) of the subject. | If the certificate has no CN, the portal shows the whole subject. |
| Type | Subject and issuer. | **Root** if the subject and the issuer are the same. Otherwise **Intermediate**. A certificate that is not self-signed also shows **Intermediate**. |
| Issuer CN | The CN of the issuer. | If the issuer has no CN, the portal shows the whole issuer. |
| Valid From | The start of the validity period. | The date only, in the date format of the browser. |
| Expires | The end of the validity period. | The date, and a status. **Valid**, **Expiring soon** when less than 30 days remain, or **Expired**. |
| SHA256 Fingerprint | The SHA-256 hash of the certificate. | The first 24 characters, then `…`. The full value shows in a tooltip. |

The fingerprint is the SHA-256 hash of the certificate, in upper case hexadecimal, with a colon
between each byte. Example form: `AB:CD:EF:...`.

Click the view icon of a row to open **Certificate Authority Details**. The details also show:

| Field | Meaning |
| --- | --- |
| Subject | The full subject, for example `CN=Example Root CA, O=Example, C=XX`. |
| Issuer | The full issuer, in the same form as the subject. |
| Serial Number | The serial number, in lower case hexadecimal, with a colon between each byte. |
| PEM Content | The text of the certificate in PEM format. |

## What the import does

1. Click **Import Certificates** or **Import**. The **Import Certificate Authorities** drawer
   opens.
2. Select the PEM file. The portal reads the file in the browser. It shows **File loaded** with
   the size of the text. If the file cannot be read, the portal shows **Failed to read the file.**
3. Click **Import**. The portal sends the text of the file to the server.
4. The server stops the import if the voter certificate policy is not **Enabled**.
5. The server finds each certificate block in the text. If there is no block, the server stops the
   import.
6. For each block, in file order, the server:
   - reads the certificate. If it cannot read the certificate, it records an error for it;
   - checks the expiry date. If the certificate is expired, it records an error for it;
   - checks the fingerprint. If the certificate is already in the election event, it counts it as
     skipped;
   - saves the certificate in the election event.
7. The server writes one entry in the electoral log for the certificates that it saved. The entry
   names the subjects of the saved certificates.
8. The portal shows the result and closes the drawer. The notifications are listed in
   [Messages and errors](#messages-and-errors).

### Messages and errors

| Message | Where | Meaning and action |
| --- | --- | --- |
| Imported \{number\} certificate(s). | Notification | The number of certificates that the server saved. |
| \{number\} certificate(s) skipped (already present). | Notification, error style | These certificates are in the election event already. No action is needed. |
| Import issues: \{errors\} | Notification, error style | Each error names the number of the block in the file. Read the error, fix the block and import again. |
| `Certificate <n>: expired on <YYYY-MM-DD>` | Part of **Import issues** | The certificate is expired. It is not imported. |
| `Certificate <n>: Failed to parse PEM: ...` | Part of **Import issues** | The block is not valid PEM. Check the lines of the block. |
| `Certificate <n>: Failed to parse X.509 certificate: ...` | Part of **Import issues** | The block is valid PEM, but not an X.509 certificate. |
| Import failed: \{error\} | Notification | The request failed. The drawer stays open. Read the message. |
| Failed to read the file. | Drawer | The browser could not read the file. Select the file again. |
| `No valid PEM certificates found in input` | Server error | The file has no block between the begin line and the end line. Check the file. |
| `Digital certificate authentication is not enabled for this election event` | Server error | Set **Voter Digital Certificate Policy** to **Enabled**, then import again. |

## Related pages

- [Certificates](../02-election-event/15-election_management_election-event_certificates.md): the **Certificates** tab of the election event.
- [Import File Formats](index.md): all import files.
