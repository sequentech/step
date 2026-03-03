---
id: admin_portal_tutorials_import-elections
title: Import/Export of Elections
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

<video controls width="100%">
  <source src="./assets/Import and Export Election.mp4" type="video/mp4" />
  Your browser does not support the video tag.
</video>

The Sequent Admin Portal provides tools to export election data for backup or auditing and to import existing election configurations to quickly set up new events.

## Exporting an Election

Exporting allows you to save a comprehensive snapshot of your election event.

![Export Menu Selection](./../assets/export_menu_selection.png)

1.  Navigate to the **Data** menu within your specific Electoral Event.
2.  Select the `Export` button.

![Export Password and Instructions](./../assets/export_menu_selections.png.png)

3.  Choose the data components you wish to include in the export:
    * **Include Voters:** Exports the registered voter list.
    * **Activity Logs:** Includes a history of administrative actions.
    * **Reports:** Includes generated election reports.
    * **Tally:** Includes the final vote counts (if available).

:::info
**Security:** If you select sensitive data like voter lists, the system automatically activates **Password Encryption** for the resulting ZIP file.
:::



![Export Password and Instructions](./../assets/export_password_display.png)

4.  Click `Export` to generate an `.ezip` file.
5.  **Save the Password:** A dialog will display a unique decryption password. Copy and store this securely; you will need it to import the file later or to unzip it manually.

## Importing an Election Event

You can import an election event using a previously exported `.ezip` file to recreate an event configuration.

![Import Election Options](./../assets/import_election.png)

1.  From the sidebar, click the **plus icon** next to "Election Events" or the `+ Create an Election Event` button.
2.  Select `Import Election Event`.

![Import File Upload](./../assets/import_file_upload.png)

3.  Drag and drop your import file or click **Browse** to select it from your local storage.
4.  If your file was encrypted, enter the **Decryption Password** you saved during the export process.
5.  (Optional) Paste an **Integrity Check (SHA-256)** hash to verify the file has not been tampered with.
6.  Click `Import`. The event will now appear in your **Active** elections list.