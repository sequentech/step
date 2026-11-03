---
id: admin_portal_reference_user_manual_users_and_roles_permissions
title: Permissions
description: "A permission lets a user see a menu item, a tab or a button, or do an action. You give permissions to roles on the Roles tab, and roles to users on the Users tab."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

A permission lets a user see a menu item, a tab or a button, or do an action. You give
permissions to roles on the **Roles** tab, and roles to users on the **Users** tab. This page
lists the permissions of version 9.0 with the label that the **Roles** tab shows.

## How permissions work

- A user gets the permissions of all their roles.
- A tab or a button that needs a permission does not show without it. The reference page of
  each screen gives the permissions of that screen.
- Many tabs need two permissions: one to see the tab, for example **View Election Event
  Keys**, and one for the data, for example **Admin Ceremony**.
- The **View ... settings** permissions of the **Settings** tabs exist, but version 9.0 does
  not use them. The **Settings** page needs **View settings** and **Edit Tenant**.
- Some actions are sensitive. Even with the permission, the admin portal asks for your password
  again. See [Before You Start](../../../../00-before-you-start.md#sensitive-actions).
- Permission labels are a different function. They limit the elections that a user can see.
  See [Permission Labels](../../../../Tutorials/admin_portal_tutorials_permission-labels.md).

## Change the permissions of a role

1. In the menu on the left, click **Users and Roles**.
2. Click the **Roles** tab.
3. Click the edit icon of the role.
4. Select or clear **Active** for the permission.

**Expected result:** the message "Permission edited" shows. See
[Roles](../Roles/admin_portal_reference_user-manual_users-and-roles_roles.md).

## List of permissions

The first column is the label on the **Roles** tab. The second column is the name of the
permission in the platform. Sequent support uses this name.

| Label | Name |
| --- | --- |
| **Admin User** | `admin-user` |
| **Admin Dashboard View** | `admin-dashboard-view` |
| **Application Export** | `application-export` |
| **Application Import** | `application-import` |
| **Create Tenant** | `tenant-create` |
| **Read Tenant** | `tenant-read` |
| **Edit Tenant** | `tenant-write` |
| **Create Election Event** | `election-event-create` |
| **Read Election Event** | `election-event-read` |
| **Edit Election Event** | `election-event-write` |
| **Delete Election Event** | `election-event-delete` |
| **Create Voter** | `voter-create` |
| **Read Voter** | `voter-read` |
| **Edit Voter** | `voter-write` |
| **Create User** | `user-create` |
| **Read User** | `user-read` |
| **Edit User** | `user-write` |
| **Create User Permission** | `user-permission-create` |
| **Read User Permission** | `user-permission-read` |
| **Edit User Permission** | `user-permission-write` |
| **Create Role** | `role-create` |
| **Read Role** | `role-read` |
| **Edit Role** | `role-write` |
| **Assign Role** | `role-assign` |
| **Create Communication Template** | `communication-template-create` |
| **Read Communication Template** | `communication-template-read` |
| **Edit Communication Template** | `communication-template-write` |
| **Read Notification** | `notification-read` |
| **Edit Notification** | `notification-write` |
| **Send Notification** | `notification-send` |
| **Read Area** | `area-read` |
| **Edit Area** | `area-write` |
| **Edit Election State** | `election-state-write` |
| **Create Election Type** | `election-type-create` |
| **Read Election Type** | `election-type-read` |
| **Edit Election Type** | `election-type-write` |
| **Read Voting Channel** | `voting-channel-read` |
| **Edit Voting Channel** | `voting-channel-write` |
| **Create Trustee** | `trustee-create` |
| **Read Trustee** | `trustee-read` |
| **Edit Trustee** | `trustee-write` |
| **Read Tally** | `tally-read` |
| **Start Tally** | `tally-start` |
| **Edit Tally** | `tally-write` |
| **Read Tally Results** | `tally-results-read` |
| **Read Publish** | `publish-read` |
| **Edit Publish** | `publish-write` |
| **Read Logs** | `logs-read` |
| **Read Tasks Execution** | `tasks-read` |
| **Read Keys** | `keys-read` |
| **Upload Documents** | `document-upload` |
| **Download Documents** | `document-download` |
| **Create Tally Sheet** | `tally-sheet-create` |
| **Trustee Ceremony** | `trustee-ceremony` |
| **Publish Tally Sheet** | `tally-sheet-publish` |
| **View Tally Sheet** | `tally-sheet-view` |
| **Admin Ceremony** | `admin-ceremony` |
| **Delete Tally Sheet** | `tally-sheet-delete` |
| **Read Cast Votes** | `cast-vote-read` |
| **Read Documents** | `document-read` |
| **Edit Documents** | `document-write` |
| **Read Support Materials** | `support-material-read` |
| **Edit Support Materials** | `support-material-write` |
| **Miru Create** | `miru-create` |
| **Miru Download** | `miru-download` |
| **Miru Send** | `miru-send` |
| **Miru Sign** | `miru-sign` |
| **Edit Contest** | `contest-write` |
| **Read Contest** | `contest-read` |
| **Edit Candidate** | `candidate-write` |
| **Read Candidate** | `candidate-read` |
| **Edit Permission Label** | `permission-label-write` |
| **Edit Scheduled Events** | `scheduled-event-write` |
| **Create Contest** | `contest-create` |
| **Delete Contest** | `contest-delete` |
| **Create Candidate** | `candidate-create` |
| **Delete Candidate** | `candidate-delete` |
| **Create Election** | `election-create` |
| **Read Election** | `election-read` |
| **Edit Election** | `election-write` |
| **Delete Election** | `election-delete` |
| **Archive Election Event** | `election-event-archive` |
| **View Election Data** | `election-data-tab` |
| **View Election Event Areas** | `election-event-areas-tab` |
| **View Election Event Data** | `election-event-data-tab` |
| **View Election Event Keys** | `election-event-keys-tab` |
| **View Election Event Logs** | `election-event-logs-tab` |
| **View Election Event Publish** | `election-event-publish-tab` |
| **View Election Event Reports** | `election-event-reports-tab` |
| **View Election Event Scheduled** | `election-event-scheduled-tab` |
| **View Election Event Tally** | `election-event-tally-tab` |
| **View Election Event Tasks** | `election-event-tasks-tab` |
| **View Election Event Voters** | `election-event-voters-tab` |
| **View Election Publish** | `election-publish-tab` |
| **View Election Voters** | `election-voters-tab` |
| **Edit Reports** | `report-write` |
| **Read Reports** | `report-read` |
| **View users and roles** | `users-menu` |
| **View settings** | `settings-menu` |
| **View templates** | `templates-menu` |
| **View election types settings** | `settings-election-types-tab` |
| **View voting channels settings** | `settings-voting-channels-tab` |
| **View templates settings** | `settings-templates-tab` |
| **View languages settings** | `settings-languages-tab` |
| **View localization settings** | `settings-localization-tab` |
| **View look and feel settings** | `settings-look-feel-tab` |
| **View truestees settings** | `settings-trustees-tab` |
| **View countries settings** | `settings-countries-tab` |
| **Import Voter** | `voter-import` |
| **View Election Event Voters Columns** | `ee-voters-columns` |
| **Manually Verify Voter** | `voter-manually-verify` |
| **View Election Event Voters Logs** | `ee-voters-logs` |
| **Export Voter** | `voter-export` |
| **View Election Event Voters Filters** | `ee-voters-filters` |
| **Delete Voter** | `voter-delete` |
| **Change Voter Password** | `voter-change-password` |
| **Election Event Localization Selector** | `election-event-localization-selector` |
| **Create Localization** | `localization-create` |
| **Read Localization** | `localization-read` |
| **Edit Localization** | `localization-write` |
| **Delete Localization** | `localization-delete` |
| **Create Area** | `area-create` |
| **Delete Area** | `area-delete` |
| **Export Area** | `area-export` |
| **Import Area** | `area-import` |
| **Upsert Area** | `area-upsert` |
| **Election Event Areas Columns** | `election-event-areas-columns` |
| **Election Event Areas Filters** | `election-event-areas-filters` |
| **Back to Election Event Tasks** | `election-event-tasks-back-button` |
| **Election Event Tasks Columns** | `election-event-tasks-columns` |
| **Election Event Tasks Filters** | `election-event-tasks-filters` |
| **Export Tasks** | `task-export` |
| **Read Application** | `application-read` |
| **Edit Application** | `application-write` |
| **Export Logs** | `logs-export` |
| **Election Event Logs Columns** | `election-event-logs-columns` |
| **Election Event Logs Filters** | `election-events-logs-filters` |
| **Election Event Scheduled Event Columns** | `election-event-scheduled-event-columns` |
| **Create Scheduled Event** | `scheduled-event-create` |
| **Delete Scheduled Event** | `scheduled-event-delete` |
| **Election Event Reports Columns** | `election-event-reports-columns` |
| **Create Report** | `report-create` |
| **Delete Report** | `report-delete` |
| **Generate Report** | `report-generate` |
| **Preview Report** | `report-preview` |
| **Election Event Monitoring Dashboard View** | `monitoring-dashboard-view-election-event` |
| **Election Monitoring Dashboard View** | `monitoring-dashboard-view-election` |
| **Monitoring Authenticated Voters** | `monitor-authenticated-voters` |
| **Read Monitoring Approve Disapprove Voters** | `monitor-all-approve-disapprove-voters` |
| **Read Monitoring Automatic Approve Disapprove Voters** | `monitor-automatic-approve-disapprove-voters` |
| **Read Monitoring Manually Approve Disapprove Voters** | `monitor-manually-approve-disapprove-voters` |
| **Read Monitoring Enrolled Overseas Voters** | `monitor-enrolled-overseas-voters` |
| **Read Monitoring Posts Already Closed Voting** | `monitor-posts-already-closed-voting` |
| **Read Monitoring Posts Already Generated Election Results** | `monitor-posts-already-generated-election-results` |
| **Read Monitoring Posts Already Opened Voting** | `monitor-posts-already-opened-voting` |
| **Read Monitoring Posts Already Started Counting Votes** | `monitor-posts-already-started-counting-votes` |
| **Read Monitoring Posts Initialized The System** | `monitor-posts-initialized-the-system` |
| **Read Monitoring Posts Started Voting** | `monitor-posts-started-voting` |
| **Read Monitoring Posts Transmitted Results** | `monitor-posts-transmitted-results` |
| **Read Monitoring Voters Voted Test Election** | `monitor-voters-voted-test-election` |
| **Read Monitoring Voters Who Voted** | `monitor-voters-who-voted` |
| **Preview Election Event Publish** | `election-event-publish-preview` |
| **Back to Election Event Publish** | `election-event-publish-back-button` |
| **Election Event Publish Columns** | `election-event-publish-columns` |
| **Election Event Publish Filters** | `election-event-publish-filters` |
| **Create Publish** | `publish-create` |
| **Regenerate Publish** | `publish-regenerate` |
| **Export Publish** | `publish-export` |
| **Start Voting** | `publish-start-voting` |
| **Pause Voting** | `publish-pause-voting` |
| **Stop Voting** | `publish-stop-voting` |
| **Publish Changes** | `publish-changes` |
| **Election Event Publish View** | `election-event-publish-view` |
| **Election Event Keys Columns** | `election-event-keys-columns` |
| **Create Ceremony** | `create-ceremony` |
| **Export Ceremony** | `export-ceremony` |
| **Election Event Tally Columns** | `election-event-tally-columns` |
| **Back to Election Event Tally** | `election-event-tally-back-button` |
| **Transmission Ceremony** | `transmition-ceremony` |
| **View IP Address** | `admin-ip-address-view` |
| **View Election Approvals** | `election-approvals-tab` |
| **View Election Event Approvals** | `election-event-approvals-tab` |
| **View Election IP Address** | `election-ip-address-view` |
| **View Election Dashboard** | `election-dashboard-tab` |
| **Export Trustees** | `trustees-export` |
| **Import Users** | `user-import` |
| **Edit voters who voted** | `voter-voted-edit` |
| **Edit voters email/phone fields** | `voter-email-tlf-edit` |
| **Edit Country Blocking Rules in Cloudflare** | `cloudflare-write` |
| **Generate Transmission Report** | `transmission-report-generate` |
## Related pages

- [Roles](../Roles/admin_portal_reference_user-manual_users-and-roles_roles.md)
- [Users](../Users/admin_portal_reference_user-manual_users-and-roles_users.md)
- [Set Up the Tenant](../../../../procedures/01-tenant.md)
