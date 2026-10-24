---
id: users_and_roles
title: Users, Roles and Role Separation
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Accounts and roles in the TOE

- **Accounts of the tenant.** Administrators, election board members and
  trustees have personal accounts in the tenant. Each account is linked to the
  person's identity at the external authentication server; the TOE stores no
  password for it.
- **Voters** have accounts in the election event, not in the tenant. They are
  created from the voters' register of the election event
  ([Tenant and Election Event Settings](05-tenant_and_event_settings.md)).
- **Roles.** A role is a named set of permissions. An account gets the
  permissions of every role it is a member of.

You manage accounts and roles in the side menu "Users and Roles". The tab
"Users" lists the accounts; "Create user" creates one, and "User Data" shows
and edits one. The tab "Roles" lists the roles; "Create role" creates one, and
"Role Data" shows and edits its permissions. The menu entry needs "View users
and roles" (`users-menu`), the "Users" tab needs "Read User" (`user-read`) and
the "Roles" tab needs "Read Role" (`role-read`).

A change of an account's roles takes effect at the account's next sign-in.

Creating, changing and deleting users, roles, role permissions and role assignments is recorded
in the electoral log of the tenant (for tenant accounts) or of the election
event (for voters).

## The roles of the evaluated configuration

The evaluated configuration has exactly these roles for people:

| Role | Who holds it | Purpose |
|---|---|---|
| `admin`, with the permissions of the next two sections removed | each administrator | the administrator role |
| "Election board", a role you create | each member of the election board, including the trustees | the election board role |
| `trustee` | each trustee, in addition to "Election board" | the trustees' key and tally ceremony steps |

No account holds the administrator role together with the election board role.
No account holds the shipped roles `admin-light` or `admin-lockdown`.

As administrator you see an election event's settings, not its election data
or its audit records. The election data (the voters' register, the candidate
list and its display data, the set of election rules and the election dates),
the election execution data and the audit records belong to the election
board: a board member imports the election data, the board publishes it in
configuration versions and reviews the audit records. The TOE refuses the
administrator's reads of election data and of audit records.

:::warning
Before the first election event is imported, check that the role `admin`
holds none of the permissions of the next two sections, and remove any that it
holds. The realm settings of the Deployment Guide have already removed them,
among them all "Sign:" permissions, "Edit Election State"
(`election-state-write`), "Create Election Event" (`election-event-create`)
and the permissions that read the voters' register and the electoral log: with
them, an account in the role `admin` could import election data, start and
end the election execution, sign protected actions alone and read the voters'
register and the audit records.
:::

### The election board's permissions

These permissions belong only to the "Election board" role (and, where
stated, to `trustee`). Remove each of them from the role `admin`:

| Function | Permissions |
|---|---|
| Import of election data | "Create Election Event" (`election-event-create`) |
| Configuration versions | "View Election Event Publish" (`election-event-publish-tab`), "View Election Publish" (`election-publish-tab`), "Read Publish" (`publish-read`), "Edit Publish" (`publish-write`), "Create Publish" (`publish-create`), "Regenerate Publish" (`publish-regenerate`), "Publish Changes" (`publish-changes`) |
| Lockdown periods | "View Election Event Scheduled" (`election-event-scheduled-tab`), "Election Event Scheduled Event Columns" (`election-event-scheduled-event-columns`), "Edit Scheduled Events" (`scheduled-event-write`), "Create Scheduled Event" (`scheduled-event-create`), "Delete Scheduled Event" (`scheduled-event-delete`); the board creates only "Start Lockdown Period" and "End Lockdown Period" |
| Areas, which are election data: the TOE refuses changes | "Read Area" (`area-read`), "Edit Area" (`area-write`), "Create Area" (`area-create`), "Delete Area" (`area-delete`), "Import Area" (`area-import`), "Upsert Area" (`area-upsert`), "Export Area" (`area-export`) |
| Sign protected actions | "Sign: initialize voting" (`sign-initialize-voting`), "Sign: open voting" (`sign-open-voting`), "Sign: close voting" (`sign-close-voting`), "Sign: generate election returns" (`sign-generate-election-returns`), "Sign: generate other election reports" (`sign-generate-reports`), "Sign: transmit results" (`sign-transmit-results`), "Sign: approve a voter manually" (`sign-approve-voter`), "Sign: approve a configuration version" (`sign-approve-configuration`) |
| Trustee steps (role `trustee` only) | "Sign: confirm custody" (`sign-key-ceremony`), "Sign: authorize tally custody" (`sign-tally-key`), "Trustee Ceremony" (`trustee-ceremony`) |
| Start, interrupt, resume and end the election execution | "Edit Election State" (`election-state-write`), "Start Voting" (`publish-start-voting`), "Pause Voting" (`publish-pause-voting`), "Stop Voting" (`publish-stop-voting`) |
| Signing requests | "Signatures: read requests" (`signing-requests-read`), "Signatures: cancel requests" (`signing-requests-cancel`), "Signatures: export requests" (`signing-requests-export`) |
| Key and tally ceremonies, counting | "View Election Event Keys" (`election-event-keys-tab`), "Read Keys" (`keys-read`), "Admin Ceremony" (`admin-ceremony`), "Create Ceremony" (`create-ceremony`), "Export Ceremony" (`export-ceremony`), "Transmission Ceremony" (`transmition-ceremony`), "View Election Event Tally" (`election-event-tally-tab`), "Read Tally" (`tally-read`), "Start Tally" (`tally-start`), "Edit Tally" (`tally-write`), "Read Tally Results" (`tally-results-read`) |
| Archiving and deletion of election events | "Archive Election Event" (`election-event-archive`), "Delete Election Event" (`election-event-delete`) |
| Audit review and monitoring | "View Election Event Logs" (`election-event-logs-tab`), "Read Logs" (`logs-read`), "Audit Electoral Log" (`electoral-log-audit`), "Browse Electoral Log" (`electoral-log-console-read`), "Query Electoral Log" (`electoral-log-console-query`), "Read Electoral Log Personal Data" (`electoral-log-personal-data-read`), "View Election Event Tasks" (`election-event-tasks-tab`), "Read Tasks Execution" (`tasks-read`), "View Monitoring Dashboards" (`monitoring-view`) |
| Read the voters' register, as a protected action where the Signatures tab requires it | "Read Voter" (`voter-read`), "View Election Event Voters" (`election-event-voters-tab`) |
| Export of the electoral log and of election execution data | "Export Logs" (`logs-export`), "Edit Reports" (`report-write`), "Export Voter" (`voter-export`), "Read Document Passwords" (`document-password-read`) |

### Permissions on election data and audit records

Also remove from `admin` every permission that reads, changes or exports
election data, election execution data or audit records. The administrator
role holds none of the following; the election board role holds only those
that the previous section and [Create the "Election board" role](#create-the-election-board-role)
name.

| Object | Permissions |
|---|---|
| Election events and elections | "Create Election Event" (`election-event-create`), "Delete Election Event" (`election-event-delete`), "Archive Election Event" (`election-event-archive`), "Create Election" (`election-create`), "Delete Election" (`election-delete`) |
| Voters' register | "Create Voter" (`voter-create`), "Read Voter" (`voter-read`), "Edit Voter" (`voter-write`), "Delete Voter" (`voter-delete`), "Import Voter" (`voter-import`), "Export Voter" (`voter-export`), "Reveal Secret Voter Fields" (`voter-secret-attribute-read`), "Edit Secret Voter Fields" (`voter-secret-attribute-write`), "Manually Verify Voter" (`voter-manually-verify`), "Change Voter Password" (`voter-change-password`), "Generate Voter Information Letter" (`voter-information-letter`), "Edit voters who voted" (`voter-voted-edit`), "Edit voters email/phone fields" (`voter-email-tlf-edit`), "Import Users" (`user-import`), "View Election Event Voters" (`election-event-voters-tab`), "View Election Voters" (`election-voters-tab`), "View Election Event Voters Logs" (`ee-voters-logs`), "Reconcile election event voter list" (`election-event-voter-list-reconciliation`), "Read Application" (`application-read`), "Edit Application" (`application-write`), "Application Import" (`application-import`), "Application Export" (`application-export`) |
| Candidate list, display data of the ballot and set of election rules | "Create Contest" (`contest-create`), "Read Contest" (`contest-read`), "Edit Contest" (`contest-write`), "Delete Contest" (`contest-delete`), "Create Candidate" (`candidate-create`), "Read Candidate" (`candidate-read`), "Edit Candidate" (`candidate-write`), "Delete Candidate" (`candidate-delete`), "Read Area" (`area-read`), "Edit Area" (`area-write`), "Create Area" (`area-create`), "Delete Area" (`area-delete`), "Import Area" (`area-import`), "Upsert Area" (`area-upsert`), "Export Area" (`area-export`) |
| Election dates and lockdown periods | "View Election Event Scheduled" (`election-event-scheduled-tab`), "Election Event Scheduled Event Columns" (`election-event-scheduled-event-columns`), "Edit Scheduled Events" (`scheduled-event-write`), "Create Scheduled Event" (`scheduled-event-create`), "Delete Scheduled Event" (`scheduled-event-delete`) |
| Data correction | "Datafix account" (`datafix-account`), which no account holds |
| Configuration versions | "View Election Event Publish" (`election-event-publish-tab`), "View Election Publish" (`election-publish-tab`), "Election Event Publish View" (`election-event-publish-view`), "Preview Election Event Publish" (`election-event-publish-preview`), "Read Publish" (`publish-read`), "Edit Publish" (`publish-write`), "Create Publish" (`publish-create`), "Regenerate Publish" (`publish-regenerate`), "Publish Changes" (`publish-changes`), "Export Publish" (`publish-export`) |
| Election execution and signing requests | "Edit Election State" (`election-state-write`), "Start Voting" (`publish-start-voting`), "Pause Voting" (`publish-pause-voting`), "Stop Voting" (`publish-stop-voting`), "Signatures: read requests" (`signing-requests-read`), "Signatures: cancel requests" (`signing-requests-cancel`), "Signatures: export requests" (`signing-requests-export`) |
| Keys, counting and results | "View Election Event Keys" (`election-event-keys-tab`), "Read Keys" (`keys-read`), "Admin Ceremony" (`admin-ceremony`), "Create Ceremony" (`create-ceremony`), "Export Ceremony" (`export-ceremony`), "Transmission Ceremony" (`transmition-ceremony`), "Trustee Ceremony" (`trustee-ceremony`), "View Election Event Tally" (`election-event-tally-tab`), "Read Tally" (`tally-read`), "Start Tally" (`tally-start`), "Edit Tally" (`tally-write`), "Read Tally Results" (`tally-results-read`), "Create Tally Sheet" (`tally-sheet-create`), "View Tally Sheet" (`tally-sheet-view`), "Review Tally Sheet" (`tally-sheet-review`), "Create Tally Sheet Import" (`tally-sheet-import-create`), "View Tally Sheet Import" (`tally-sheet-import-view`), "Review Tally Sheet Import" (`tally-sheet-import-review`), "Execute Tally Recount" (`tally-recount-execute`), "Submit tally resolution" (`tally-resolution-submit`), "Generate Transmission Report" (`transmission-report-generate`), "Miru Create" (`miru-create`), "Miru Download" (`miru-download`), "Miru Send" (`miru-send`), "Miru Sign" (`miru-sign`), "Read Results Publication" (`publish-results-read`), "Edit Results Publication" (`publish-results-write`) |
| Reports and documents | "View Election Event Reports" (`election-event-reports-tab`), "Read Reports" (`report-read`), "Edit Reports" (`report-write`), "Create Report" (`report-create`), "Generate Report" (`report-generate`), "Delete Report" (`report-delete`), "Preview Report" (`report-preview`), "Read Documents" (`document-read`), "Download Documents" (`document-download`), "Upload Documents" (`document-upload`), "Edit Documents" (`document-write`), "Read Document Passwords" (`document-password-read`) |
| Audit records and monitoring | "View Election Event Logs" (`election-event-logs-tab`), "Read Logs" (`logs-read`), "Export Logs" (`logs-export`), "Audit Electoral Log" (`electoral-log-audit`), "Browse Electoral Log" (`electoral-log-console-read`), "Query Electoral Log" (`electoral-log-console-query`), "Read Electoral Log Personal Data" (`electoral-log-personal-data-read`), "View Election Event Tasks" (`election-event-tasks-tab`), "Read Tasks Execution" (`tasks-read`), "Export Tasks" (`task-export`), "View Monitoring Dashboards" (`monitoring-view`), "Configure Monitoring Dashboards" (`monitoring-configure`), "View Election IP Address" (`election-ip-address-view`), "View IP Address" (`admin-ip-address-view`) |

The administrator role keeps the other permissions of `admin`. Among them are
the user and role permissions; the Signatures tab's permissions for protected
actions, trusted issuers, certificate checks and certificate registrations;
"Edit Tenant" (`tenant-write`) and "View settings" (`settings-menu`); "Read
Election Event" (`election-event-read`), "Edit Election Event"
(`election-event-write`), "Read Election" (`election-read`), "Edit Election"
(`election-write`), "View Election Event Data" (`election-event-data-tab`) and
"View Election Data" (`election-data-tab`), for the settings of
[Tenant and Election Event Settings](05-tenant_and_event_settings.md); in these
forms the TOE shows you only the settings and refuses changes of election data.
They also include `attack-detection-write`, for the attack detection rates
([Audit, Monitoring and Notifications](09-audit_and_monitoring.md#attack-detections));
the trustee registration permissions; "View Election Dashboard"
(`election-dashboard-tab`), where your Dashboard shows the election event's
phase and settings, not election execution data such as turnout, cast votes or
ballot boxes; and the self-test permissions `self-test-settings-write`,
`self-test-run` and `self-test-read`.

No role holds "Edit Results Publication" (`publish-results-write`), neither the
administrator role nor the election board role. No account holds "Datafix
account" (`datafix-account`): its data-correction routes change the voters'
register and the voting record outside the import, and the evaluated
configuration does not use them. No role holds "Manually Verify Voter"
(`voter-manually-verify`): its link signs a voter in without the external
authentication server.

Only the "Election board" role holds "Delete Election Event"
(`election-event-delete`) and "Archive Election Event"
(`election-event-archive`). Deleting an election event also deletes its audit
records, so the TOE refuses every deletion of an election event unless its
election execution data has been exported and the required authorisations are
reached. An election event imported with wrong election data is archived by
the election board, not deleted
([Tenant and Election Event Settings](05-tenant_and_event_settings.md#import-of-election-data)).

### Create the "Election board" role

1. In "Users and Roles", open the tab "Roles" and select "Create role". Name
   the role, for example `Election board`.
2. In "Role Data", give it these permissions:
   - the board functions: "Create Election Event" (`election-event-create`);
     the configuration version permissions of the table above; the lockdown
     period permissions of the table above; every "Sign:"
     permission of the table above except the trustee steps; "Edit Election
     State" (`election-state-write`), "Start Voting" (`publish-start-voting`),
     "Pause Voting" (`publish-pause-voting`), "Stop Voting"
     (`publish-stop-voting`); "Signatures: read requests"
     (`signing-requests-read`), "Signatures: cancel requests"
     (`signing-requests-cancel`), "Signatures: export requests"
     (`signing-requests-export`); "Admin Ceremony" (`admin-ceremony`), "Create
     Ceremony" (`create-ceremony`), "Export Ceremony" (`export-ceremony`),
     "Transmission Ceremony" (`transmition-ceremony`), "Read
     Tally" (`tally-read`), "Start Tally" (`tally-start`), "Edit Tally"
     (`tally-write`), "Read Tally Results" (`tally-results-read`); "Read
     Logs" (`logs-read`), "Export Logs" (`logs-export`), "Edit Reports"
     (`report-write`), "Export Voter" (`voter-export`), "Audit Electoral Log" (`electoral-log-audit`), "Browse
     Electoral Log" (`electoral-log-console-read`), "Query Electoral Log"
     (`electoral-log-console-query`); `self-test-run` and `self-test-read`;
     "Delete Election Event" (`election-event-delete`), "Archive Election
     Event" (`election-event-archive`);
   - read-only access to the accounts and roles, so that the board can check
     the role separation: "View users and roles" (`users-menu`), "Read User"
     (`user-read`), "Read Role" (`role-read`) and "Read User Permission"
     (`user-permission-read`);
   - the read and view permissions of the screens the board uses: "Read
     Election Event" (`election-event-read`), "Read Area" (`area-read`), "Read Voter" (`voter-read`), "View Election
     Event Voters" (`election-event-voters-tab`), "View Election Event Data"
     (`election-event-data-tab`), "View Election Data" (`election-data-tab`),
     "View Election Event Keys" (`election-event-keys-tab`), "View Election
     Event Tally" (`election-event-tally-tab`), "View Election Event Publish"
     (`election-event-publish-tab`), "View Election Publish"
     (`election-publish-tab`), "View Election Event Logs"
     (`election-event-logs-tab`), "View Election Event Tasks"
     (`election-event-tasks-tab`), "Read Tasks Execution" (`tasks-read`),
     "Election Event Signatures Tab" (`election-event-signatures-tab`),
     "Signatures: read protected actions" (`signing-rules-read`), "Signatures:
     read certificates" (`signing-certificates-read`), "Read Publish"
     (`publish-read`), "Read Keys" (`keys-read`), "View Election Dashboard"
     (`election-dashboard-tab`), "View Monitoring Dashboards"
     (`monitoring-view`).
3. Give "Read Electoral Log Personal Data" (`electoral-log-personal-data-read`),
   "Read Document Passwords" (`document-password-read`) and "Reveal Secret
   Voter Fields" (`voter-secret-attribute-read`) only if the election
   organizer decides that the board needs them, and then only to the
   "Election board" role.
4. Do not give the role any permission to create or change users, roles,
   elections, contests, candidates, areas, voters, signing
   rules, trusted issuers, certificate checks or self-test settings, and never
   "Edit Results Publication" (`publish-results-write`). The board creates
   election events only by importing signed configuration packages. Each
   such import writes every part of the package itself and needs no other
   permission of the importing member.
5. Check that "Export Logs" (`logs-export`) and "Edit Reports"
   (`report-write`) are both in the role: exporting the electoral log needs
   both.

:::warning
Give "Signatures: cancel requests" (`signing-requests-cancel`) to the whole
"Election board" role. Without it, only the member who started a signing
request can abort it, and the other members cannot stop an action they object
to.
:::

### The trustee role

Use the shipped role `trustee` for trustees. It contains "Trustee Ceremony"
(`trustee-ceremony`) and the read permissions trustees need. In the evaluated
configuration the protected actions "Confirm custody (key ceremony)" and
"Authorize tally custody (tally)" need the trustees' signatures
([Configuring Protected Actions](06-signing_rules.md)): add "Sign: confirm
custody" (`sign-key-ceremony`) and "Sign: authorize tally custody"
(`sign-tally-key`) to `trustee`.

:::note
The side menu shows holders of "Trustee Ceremony" the entry "Trustee
Dashboard". It is not used in the evaluated configuration: trustees perform
their key and tally steps only with the trustee tool of the TOE delivery on an
offline workstation, as the Handbook for Election Organizers describes, and
sign their custody steps as protected actions.
:::

### Registering the trustees' public keys

Before the key ceremony, register each trustee in Settings > "TRUSTEES" with
"Create Trustee". You need "Create Trustee" (`trustee-create`), "Read Trustee"
(`trustee-read`) and "Edit Trustee" (`trustee-write`).

1. Receive the trustee's two public keys from the trustee in person, or on a
   medium the trustee hands over: `signing_public_key` and
   `share_encryption_public_key`, as the trustee tool prints them.
2. Enter the trustee's name, and paste the two keys into the two key fields.
   Their helper texts name them: "Signing public key from trustee
   public-keys" and "Share encryption public key from trustee public-keys".
3. Check the registered keys together with the trustee, character by
   character.
4. In the trustee's personal account, select this trustee name in "Act as
   Trustee".

:::warning
A wrong public key lets someone other than the trustee act for the trustee's
share, or stops the key ceremony. Never take the keys from an email or a chat
message, and never change a registered trustee once the key ceremony has
started.
:::

## Personal accounts

Create one personal account for each administrator, each board member and each
trustee whom the election organizer has appointed in writing.

1. In "Users and Roles", tab "Users", select "Create user".
2. Enter the person's "Username" and "Email" exactly as the external
   authentication server knows them: at the person's first sign-in, the TOE
   links the sign-in to the existing account with the same username or email.
   The TOE also sends its email notifications to the administrators and the
   election board at these addresses.
3. Tick "Enabled".
4. Make the account a member of exactly one of the roles of the table above:
   the administrator role, or "Election board". For a trustee, also add
   `trustee` and, in "Act as Trustee", select the trustee whose key share the
   person holds.
5. Check the summary in "Review changes" and confirm.
6. Ask the person to sign in once. The external authentication server must
   accept them with two factors, and the TOE must link the sign-in to this
   account. If the person ends up in an account you did not create, or in none,
   stop and check the authentication settings
   ([Authentication and Session Settings](08-authentication_and_sessions.md)).

:::warning
Never create an account for a person the election organizer has not
appointed, and never give a board member the administrator role "for a short
time". Role separation is a requirement of the evaluated configuration at all
times.
:::

## Accounts that come with the deployment

The person who runs the servers creates the customer's tenant as the
Deployment Guide describes, adds the first personal administrator account to
it, and gives you the tenant name that you enter under "Select Tenant" when you
sign in ([Signing in](03-admin_portal.md#signing-in)). Give the same tenant name
to every person for whom you create an account. The super-admin tenant, in
which the TOE was installed, holds no election event and no enabled personal
account: nobody works in it.

Besides the first personal administrator account, the customer's tenant
contains accounts that are not personal accounts:

- **The seeded account `admin`.** It is created with the tenant, has no
  password and holds the permissions of the seeded administrator group. As
  soon as your own personal administrator account works, delete it: in
  "Users", open it, delete it and confirm "Are you sure you want to delete
  this user?". Check that it no longer appears in the "Users" list.
- **The trustees' service accounts `trustee1` and `trustee2`.** They are the
  machine identities of the two instances of the trustee server. The
  Deployment Guide gives them random passwords. They are not personal
  accounts: never sign in with them, never use them for a board member, and do
  not delete or change them.
- **Technical service accounts of the TOE** (their names start with
  `service-account-`). Do not change or delete them.

:::danger
Until you delete it, the seeded account `admin` holds the user, role and
permission label permissions, with which an account can be given any
permission. Delete it before you create the accounts of the election board.
:::

## When a person leaves or changes function

When the election organizer tells you that a person no longer holds their
function, act on the same day:

1. In "Users", open the person's account, untick "Enabled" and confirm. A
   disabled account cannot sign in again.
2. Remove the account from all its roles.
3. Ask the organisation that runs the external authentication server to
   withdraw the person's credentials there.
4. If the person was a board member or trustee, revoke the registration of
   their signing certificate on the same day: on the Signatures tab of every
   election event of the tenant, "Certificates", "Registered certificates",
   select "Revoke" for each certificate registered to the person and enter the
   "Reason"
   ("Signatures: revoke certificates", `signing-certificates-revoke`). A
   registration cannot be deleted. A revoked certificate cannot sign any more; signatures it already made still
   count, and incomplete signing requests it signed are cancelled ("A
   certificate that signed it was revoked"). Inform the election board. A
   signing request the person had started stays waiting until it is completed,
   cancelled or expires: the board cancels it if needed
   ([Configuring Protected Actions](06-signing_rules.md)).
5. Check that the number of remaining board members who can sign each
   protected action is still at least its "Signatures needed". If not, the
   election organizer appoints a new member before the next action is due.
6. After the election event's retention period, delete the account as the
   election organizer's regulation requires.

If a person changes from the election board to the administrators, or the other
way round, create a new account for the new function and disable the old one,
so that the electoral log shows clearly which function performed each action.

:::note
During a lockdown period the TOE refuses changes to the voters of the election
event. Lockdown does not block changes to tenant accounts, so disable the
account of a person who leaves at once, also during the execution phase.
:::

## Permission labels

A permission label restricts what an account can reach to the elections that
carry the same label. Labels are set with "Permission Label" in the account's
data and need "Edit Permission Label" (`permission-label-write`). Use them only
as the election organizer decides. After any change, check that every board
member can still reach each election whose protected actions they sign.
