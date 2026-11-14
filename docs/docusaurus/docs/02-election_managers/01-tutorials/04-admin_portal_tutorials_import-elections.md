---
id: admin_portal_tutorials_import-elections
title: Import/Export of Elections
description: "The Sequent Admin Portal provides tools to export election data for backup or auditing and to import existing election configurations to quickly set up new events."
---

import GoogleVideo from '@site/src/components/GoogleVideo';

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

<GoogleVideo id="13KrQtqIfsw24sZCCBjO0-f6wM_kcDUI-" />

The Sequent Admin Portal provides tools to export election data for backup or auditing and to import existing election configurations to quickly set up new events.

## Exporting an Election Event

Exporting allows you to save a comprehensive snapshot of your election event.

![Export Menu Selection](./assets/export_menu_selection.png)

1.  Open the **Data** tab of the election event.
2.  Click **Export**.

![Export Password and Instructions](./assets/export_menu_selections.png)

3.  Choose the data components you wish to include in the export:
    * **Include Voters:** Exports the registered voter list.
    * **Activity Logs:** Includes a history of administrative actions.
    * **Bulletin Board:** Includes the cryptographic state of the Election Event such as key ceremonies.
    * **Publications:** Includes the publication history for the Election Event.
    * **S3 Files:** Includes images, support materials or other files that are saved in cloud storage.
    * **Scheduled Events:** Includes configured Scheduled Events.
    * **Reports:** Includes generated election reports.
    * **Applications:** Includes the enrollment applications of voters.
    * **Tally:** Includes the final vote counts (if available).
    * **Certificates:** Includes the certificate authorities of the election event.

:::info
**Security:** Select **Encrypt with Password** to protect the file with a password. If you select **Bulletin Board**, **Reports** or **Applications**, the file is always protected with a password. Select **Encrypt with Password** to also include the decrypted secret voter fields.
:::



![Export Password and Instructions](./assets/export_password_display.png)

1.  Click **Export**. A password-protected export is an `.ezip` file.
2.  **Save the Password:** A dialog will display a unique decryption password. Copy and store this securely; you will need it to import the file later or to unzip it manually.

## Importing an Election Event

You can import an election event from a `.json` file, a `.zip` export or a password-protected `.ezip` export. For the steps, see [Create the Election Event](../03-procedures/02-event.md#import-an-election-event-alternative).

:::info
You can only import election events exported from the same major version, with the same or lower minor version.
For example, with version **10.0** installed, you can import events exported from version **10.0**, but not from version **9.x** or **10.1**.
:::


![Import Election Options](./assets/import_election.png)

1.  From the sidebar, click the **+** icon next to **Election Events**, or **Create an Election Event** at the end of the tree.
2.  Click **Import Election Event**.

![Import File Upload](./assets/import_file_upload.png)
