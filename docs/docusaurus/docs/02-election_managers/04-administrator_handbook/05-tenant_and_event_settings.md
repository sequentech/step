---
id: tenant_and_event_settings
title: Tenant and Election Event Settings
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Order of work

To complete the tenant and election event settings:

1. Once per tenant, before a board member creates the first election event: set
   up signed configuration packages
   ([Signed configuration packages](#signed-configuration-packages)).
2. Once a member of the election board has created the empty approval event,
   configure **Import election data**, the trusted issuers and the board members'
   certificate registrations on its tab "Signatures"
   ([The approval event](#the-approval-event)).
3. In the preparation phase, a member of the election board imports the
   election data of each election event as a signed configuration package
   ([Import of election data](#import-of-election-data)).
4. After import, check the administrator settings you can see. Ask the board
   to confirm the remaining event and election values against the tables of
   [Configuration the board must verify](#configuration-the-board-must-verify) and
   [Settings of each election](#settings-of-each-election). The signed package must
   already carry the required values.
   If one differs, use [Correcting imported election
   data](#correcting-imported-election-data) before continuing.
5. Set the protected actions
   ([Configuring Protected Actions](06-signing_rules.md)), the automatic self-tests ([Security and Self-Tests](07-self_tests.md)) and check the
   authentication settings
   ([Authentication and Session Settings](08-authentication_and_sessions.md))
   of the election event.
6. Tell the election board that the settings are complete. After completing
   the key ceremony, the board publishes the configuration version and approves it
   ([Publishing the configuration version](#publishing-the-configuration-version)).

All of these steps except the first belong to the preparation phase.

## Signed configuration packages

### What the TOE checks

The election organizer delivers the election data (the voters' register, the
candidate list and its display data, the set of election rules and the
election dates) as a **signed configuration package**: an election event file
with a manifest that the election organizer signs, and with the approvals of
election board members. With the tenant setting `configuration_signing` set as
below, the TOE admits the import of an election event only if:

- the package's signing certificate chains to a CA certificate in
  `package_roots`, and the manifest signature is valid;
- every file of the package matches the manifest: changed, missing or extra
  files are refused;
- the package carries at least `required_approvals` approvals by distinct
  people whose certificates chain to a CA certificate in `staff_roots`;
- the manifest is not dated more than 5 minutes in the future;
- the same package has not been imported before.

Every error stops the import. An unsigned file is refused (`package.unsigned`),
a package with too few approvals is refused (`package.too-few-approvals`), and
a package that was already imported is refused (`package.already-imported`).
While `configuration_signing` is missing, its `policy` is not `required` or
`required_approvals` is not set, the TOE refuses every import. The TOE records
each successful import of a signed package in the electoral log of the new
election event.

### Required values

These are the fields of the tenant setting `configuration_signing`. On the screen
of [Configure certificates in the admin portal](#configure-certificates-in-the-admin-portal),
`package_roots` is **Package-signing CAs**, `staff_roots` is **Election board
approval CAs** and `required_approvals` is **Required distinct approvals**; the
screen sets `policy` to `required` itself:

| Field | Meaning | Default | Required value |
|---|---|---|---|
| `policy` | `required`: only signed packages with enough approvals are imported; with any other value the TOE refuses every import | not set: every import is refused | `required` |
| `package_roots` | the CA certificates, in PEM format, that issue the election organizer's package signing certificates | none | the election organizer's CA certificates for package signing |
| `staff_roots` | the CA certificates, in PEM format, that issue the certificates with which election board members approve a package | none | the election organizer's CA certificates for package approvals |
| `required_approvals` | the number of approvals by distinct members of the election board that a package needs | not set: every import is refused | at least 2, as the election organizer decides |

Get the CA certificates from the election organizer through a channel that
protects their integrity, for example in person, and compare their
fingerprints with the ones the organizer states in writing.

:::warning
Until `configuration_signing` is set with `policy` `required`, the CA
certificates and `required_approvals`, the election board cannot import any
election event. Set it before a board member creates the first election event:
while an election event that is not archived exists in the tenant, the TOE refuses
the save. The TOE refuses a `required_approvals` below 2: with fewer approvals, one
board member could bring in election data that no other member has checked.
:::

### Configure certificates in the admin portal

To use this screen you need the permissions **Edit Tenant** (`tenant-write`)
and **View settings** (`settings-menu`). The TOE accepts the change only from an
account with the permission **Admin User** (`admin-user`), and only for the
account's own tenant. To configure the certificates:

1. Open **Settings → Configuration packages** before a board member creates the
   first election event. The screen is titled **Signed configuration packages**.
2. Under **Package-signing CAs**, select **Choose CA certificate file** and open
   the organizer's public PEM CA file. A file can contain several CA
   certificates, including an approved successor. Do not select a private key
   or a board member's personal certificate.
3. Check the name, expiry date and full **SHA-256** fingerprint of **every**
   displayed certificate against the values the organizer stated in writing.
   Select the confirmation checkbox only when they all match.
4. Repeat these checks under **Election board approval CAs**, using the CA file
   for board members' approval certificates.
5. Set **Required distinct approvals** to the organizer's approved number,
   **at least 2**. Confirm the count and the package-signing CAs carefully:
   these values cannot change after the first save.
6. Select **Save certificate settings**. Continue only after
   **Certificate settings saved and read back successfully.** Reload the page
   and check the saved certificates and approval count before telling the board
   that imports can begin.

![Configuration packages form showing public CA details and fingerprint confirmations](assets/configuration-packages.png)

*Illustration: the form with synthetic CA certificates. The certificates shown
are examples, not trust anchors for an evaluated installation.*

The screen accepts public PEM CA files up to **256 KiB**. It rejects malformed,
expired, not-yet-valid, duplicate and personal certificates. Certificate names
and fingerprints are calculated in the browser from their DER bytes; no private
key is needed or sent. Saving changes only `configuration_signing`, preserves
other tenant settings and reads the result back from the TOE. An intervening
administrator's change causes the save to fail rather than overwrite their
configuration.

### Expected results and recovery

| What you see | What to do |
|---|---|
| **Save certificate settings** is unavailable | Supply both CA files, compare every fingerprint and confirm both sets; choose a whole approval count of at least 2. |
| A certificate-file error | Obtain the correct public PEM CA file. Do not bypass a fingerprint mismatch or use a private key. An expired saved package-signing CA remains fixed: tell the election organizer and the operator. This screen cannot replace it, and a new tenant ([Create the customer's tenant](../../05-reference/06-deployment_guide/08-installing_toe.md#create-the-customers-tenant)) is the only way to a new one. |
| **Settings were not confirmed** | Reload before retrying. Check your role and tenant, whether another administrator changed the settings, and whether the tenant holds an election event that is not archived. Do not tell the board that configuration succeeded. |
| Package-signing CAs or the approval count are disabled | They were already configured. Do not try to change them through the tab "Backup / Restore" or a direct API call. |
| Replacing the **Election board approval CAs** is refused | **Election board approval CAs** (the notice on the screen calls them "Staff CAs") can change only while every election event of the tenant is archived or deleted. An election event can be archived only in the preparation phase ([Correcting imported election data](#correcting-imported-election-data)); one that has left that phase stays until the board deletes it after the export. Confirm this with the election board; never archive or delete an election event just to enable a certificate change. |

The TOE enforces the required policy, the approval-count bounds, the public PEM
format and the change restrictions also for changes made without this screen
and for tenant restores.
The TOE performs the authoritative X.509 trust-chain and signature checks when
it imports a package. The browser's successful certificate preview alone does
not authorize an election-data import.

Set this configuration once, before a board member creates the first election
event. Include the approved current and successor CAs from the start.

## Import of election data

A member of the election board imports the election data; you do not. The
TOE accepts election data only as a signed configuration package,
through the entry "Import Election Event". Each import creates a new election
event in the preparation phase. The import needs the permission "Create Election Event"
(`election-event-create`), which only the role `election-board` holds, and the
package must carry the approvals of at least `required_approvals` different
board members, made with certificates that chain to `staff_roots`. The board
members make their approvals outside the TOE, and the TOE verifies them when
the package is imported. The board's procedure, with the digest in the field
"Integrity Check (SHA-256)", is in
[Importing the election data](../05-election_organizer_handbook/06-preparation.md#importing-the-election-data).

Your part:

1. Before a board member creates the first election event, set
   `configuration_signing`
   ([Signed configuration packages](#signed-configuration-packages)).
2. Before the package is approved, give the election organizer the required
   Data-tab values in the tables of
   [Settings of the election event](#settings-of-the-election-event) and
   [Settings of each election](#settings-of-each-election). After each import, when
   the election board names the new election event, check the controls of your form
   in **Data → Security settings**, ask the board to check the rest, and leave them
   unchanged. You see the settings, not the election data. Tell the operator that
   an election event was imported.
3. If the election board reports that an import was refused although the
   package is the one the election organizer released, open **Settings → Configuration packages** and check that the saved CA
   fingerprints and approval count still match your approved values. Do not change them. Treat the refusal as a security-relevant event
   ([Security-Relevant Events for Administrators](11-security_relevant_events.md)).

Use the complete signed-package procedure for the evaluated configuration.
The TOE also requires the board's authorisation and the right phase for separate
imports of voters, areas, candidates and schedules, and refuses direct changes
of this data.

Some settings available to administrators also affect the ballot design that
is approved in the signed package. The election organizer includes the required
event and election Data-tab values before the package approvals. Administrators
check the values their form shows and the board the others; they do not change
them to repair a package.
Otherwise publication can refuse the changed ballot design. The tenant settings,
protected-action rules, certificate registrations and the automatic self-tests are
configured separately as their procedures describe. Areas remain election data.

### The approval event

Each import also needs the board's online approval: a signing request of the
protected action **Import election data**. Protected actions, trusted issuers
and certificate registrations belong to an election event, and the election
event that an import creates does not exist before the import. The board
therefore approves imports in an existing election event in the preparation
phase, the approval event. Before the first import there is none, so the board
creates an empty one
([Prepare the first import's approval event](../05-election_organizer_handbook/06-preparation.md#prepare-the-first-imports-approval-event)).

| Who | What |
|---|---|
| A board member | Creates the empty approval event with **Create an Election Event**. It holds no election data and never opens voting. |
| You | On its **Signatures** tab, configure **Import election data**, the trusted issuers and the board members' certificate registrations ([Configuring Protected Actions](06-signing_rules.md)). |
| The importing board member | Selects it in **Board approval for import**. The exact uploaded file then needs the approval event's board signatures as well as the package checks. |

### Correcting imported election data

If the imported election data is wrong, nobody corrects it in the TOE:

1. The election organizer issues a corrected signed configuration package.
2. A member of the election board imports it. It becomes a new election event.
3. The election board archives the wrong election event in the preparation
   phase with the entry "Archive this Election Event" of its menu. An archived election event never
   opens voting; its data and its audit records stay. It is never unarchived.
4. Check the settings of the new election event as after every import.

:::danger
Never delete an election event, and never ask for one to be deleted. Deleting
an election event also deletes its audit records. Only the election board
deletes election events, after the export of the election execution data and
with the required authorisations; the TOE refuses every other deletion.
:::

## Settings of the election event

![Security settings on the Data tab, from Key ceremony scope to Seal record access](assets/event-advanced-settings.png)

*Illustration: the administrator’s Security settings on the Data tab in a
synthetic v10.0.0 election event. Use the required values below, and verify the
signed package’s settings; do not copy a sample election event’s state or
identifiers.*

Open the election event and select **Data → Security settings**. This
administrator form is writable only during preparation. It exposes these
controls without exposing the rest of the ballot design, the voters' register or
the election dates:

| Control | What to check |
| --- | --- |
| **Key ceremony scope** | The value of the signed package, as the election organizer states it in writing. |
| **Trustee participation** | **Trustees participate**; the evaluated configuration uses manual custody. |
| **Ballot encryption** | The value of the signed package, as the election organizer states it in writing. |
| **Decoded ballots in results** | **Do not include**. |
| **Voter ballot signatures** | The value of the signed package, as the election organizer states it in writing. This differs from ballot-box-signed receipts and from voter digital certificates. |
| **Ballot box sealing** | **Seal when voting closes**. |
| **Seal record access** | **Election board only**. |
| **Date and time** | The zones of the signed package, as the election organizer states them in writing. They do not change the signed election dates, but they belong to the approved ballot design. |

These controls belong to the ballot design that the signed package approved, so a
saved change makes publication refuse that design. Do not use **Save security
settings** on an imported election event to repair a value: if a control differs
from this table or from the election organizer's written value, follow
[Correcting imported election data](#correcting-imported-election-data). Do not add
a board permission to make another form available.

Your controls **Trustee participation**, **Ballot box sealing** and **Seal record
access** are the settings "Keys/Tally Ceremonies Policy", "Ballot Box Seal Policy"
and "Ballot Box Seal Record" of the board's view.

### Configuration the board must verify

The following table specifies the values the organizer must include in the
signed package. Give it to the board and obtain its confirmation. Some fields
are election data and are deliberately absent from the administrator form.
The section names below identify the board's read-only **Data** view.

| Setting (section) | Default | Required value | Why |
|---|---|---|---|
| "Scheduled close without signatures" ("Voting lifecycle"): "Refuse" / "Run as system" | "Refuse" | "Refuse" | A scheduled close that is not part of the configuration version the election board approved does not run; the board's signers close voting. |
| "Show User Profile" ("Ballot Design"): a switch | not set, which counts as on | off | The voting portal then shows voters no link to account functions, which the evaluated configuration does not offer. |
| "Show Cast Vote Logs Tab" ("Ballot Design"): "Show Cast Vote Logs Tab" / "Hide Cast Vote Logs Tab" | "Hide Cast Vote Logs Tab" | "Hide Cast Vote Logs Tab" | Audit records are shown only to the election board. With "Hide", the TOE also refuses voters' requests for the log. |
| "Voting Channels Allowed" ("Voting Channels Allowed"): "Online", "Kiosk", "Early voting", "Telephone voting" | "Online" only | "Online" only | The evaluated configuration covers online voting in the execution phase. Early voting accepts ballots before the execution phase. The telephone voting channel is not configured: the TOE has no telephone provider and no credentials for its endpoints, which refuse every request. |
| "Receipts signed by the ballot box" ("Ballot Receipts"): "Disabled" / "Signed by the ballot box" | "Disabled" | "Signed by the ballot box" | The ballot box stores and signs each ballot, and the voter sees the Ballot ID only once the ballot has been received: the voter's feedback that the vote was stored. |
| "Period for checking cast ballots" ("Ballot Receipts"): "No limit" / "Until a date", with "Checks available until" | "No limit" | "Until a date", with "Checks available until" set to the end of the election period | Voters check that their ballot was recorded only during the execution phase. |
| "Lockdown Status" ("Advanced Configurations"): "Locked Down" / "Not Locked Down" | "Not Locked Down" | not set here | It changes only through the scheduled events "Start Lockdown Period" and "End Lockdown Period" ([Modes of Operation and Failure Handling](10-modes_of_operation.md)). |
| "Ballot Box Seal Policy" ("Advanced Configurations"): "Seal at close" / "Do not seal" | "Do not seal" | "Seal at close" | When voting closes, the TOE seals each ballot box: a closed ballot box cannot be reopened, and any later change to it is detected. |
| "Ballot Box Seal Record" ("Advanced Configurations", shown with "Seal at close"): "Restricted" / "Public" | "Restricted" | "Restricted" | With "Public", anyone who knows the election event's identifiers can download the seal record without signing in. |
| "Keys/Tally Ceremonies Policy" ("Advanced Configurations"): "Allow Automatic Ceremonies" / "Manual Ceremonies" | "Manual Ceremonies" | "Manual Ceremonies" | Every trustee must confirm custody of their key share at the key ceremony, and the tally ceremony starts only with the trustees' authorisations. |
| "Batch Anonymity Policy" ("Advanced Configurations"): "Warn" / "Refuse" | "Warn" | "Refuse" | The TOE stops a tally when an area's ballot batch has fewer than five voters, or differs from the batch another tally already posted for the same area and contest, so that no small batch reveals how voters voted. |
| "Voter Digital Certificate Policy" ("Advanced Configurations"): "Enabled" / "Disabled" | "Disabled" | "Disabled" | Voters authenticate only at the external authentication server. |
| "Enrollment" ("Advanced Configurations", "Voter Authentication"): "Enabled" / "Disabled" | not set, which counts as "Enabled" | "Disabled" | Only voters in the voters' register can log in; voters cannot register themselves. |
| "OTP" ("Advanced Configurations", "Voter Authentication"): "Enabled" / "Disabled" | not set, which counts as "Enabled" | "Disabled" | The TOE offers voters no one-time-code form of its own; the second factor belongs to the external authentication server. |

Notes:

- The settings of the section "Voting lifecycle" are part of the election
  event's configuration: the configuration approval signs them, and scheduled
  openings and closings follow the stricter of the current and the published
  settings.
- "Ballot Box Seal Policy" cannot be changed once voting has opened: the TOE
  refuses with "The Ballot Box Seal Policy can't be changed after voting has
  opened." "Ballot Box Seal Record" is locked at the same time.
- "Receipts signed by the ballot box" cannot be changed once voting has
  started. The required
  value must already be in the signed package; a different imported value
  requires a corrected package and a new election event.
- Ask the board to check the field "Checks available until" in the time zone that
  the field shows.
  It must match the end of the election period as the election organizer
  states it to you in writing, with its time zone.
- "Show User Profile", "Enrollment" and "OTP" count as on or "Enabled" when
  their values are not set. The signed package must contain explicit off or
  "Disabled" values. Ask the board to reopen the imported event to check them; do
  not toggle these controls to repair an incomplete package.
- The setting "Results Website" is not one of your settings: the admin portal
  shows it only to accounts with the permission "Edit Results Publication"
  (`publish-results-write`), which no role holds. The section "Publish to results
  website" is therefore never available.
- An imported package can carry any value of these settings. Ask the board to check
  all of them after every import, even when the previous election event was correct.

:::warning
Never select the option "Allow Automatic Ceremonies" (on your form: the option
"Automated ceremonies" of the control "Trustee participation"). It makes the switch
"Automatic Ceremony" available when a key ceremony is created, and an automatic ceremony
removes the trustees' custody confirmation and their authorisation of the
tally: the votes could then be decrypted without the trustees' consent. The
evaluated configuration forbids automatic ceremonies. The switch "Automatic
Ceremony" of each key ceremony decides whether the trustees confirm custody,
whatever this policy says. The election board, which creates
the key ceremony ("Create Key Ceremony" in the election event's tab "Keys"), never switches on
"Automatic Ceremony" and checks that every trustee confirmed custody. Check
this policy after every import, and again before the key ceremony.
:::

:::warning
With "Do not seal", closed ballot boxes can be reopened and are not protected
by a seal. With receipts "Disabled", voters get no signed confirmation that the
ballot box stored their vote. With "No limit", voters can still check ballots
after the election period has ended. With "Run as system", voting closes at a
scheduled time that nobody authorised. With "Enrollment" or "OTP" set to
"Enabled", or "Show User Profile" on, voters are offered functions of the TOE
before or beside the external authentication server that the evaluated
configuration excludes. With "Show Cast Vote Logs Tab", voters can read
records of the electoral log.
:::

:::warning
No results website is part of the evaluated configuration: the results portal is
not deployed
([Keep the results and kiosk portals off](../../05-reference/06-deployment_guide/07-hardening.md#keep-the-results-and-kiosk-portals-off)).
Never give any role the permission "Edit Results Publication"
(`publish-results-write`): without it nobody can configure or publish the results
website, whatever a package contains.
:::

## Settings of each election

Ask the board to open each election's **Data** tab and confirm the following
values. Your form on the tab "Data" shows none of these fields. If a value is
wrong, ask the election organizer for a corrected signed configuration package
([Correcting imported election data](#correcting-imported-election-data)); do not
ask for election-data permissions.

| Setting (section) | Default | Required value | Why |
|---|---|---|---|
| "Voting Channels Allowed" ("Voting Channels Allowed"): "Online", "Kiosk", "Early voting", "Telephone voting" | as the election event | "Online" only | as for the election event |
| "Audit Button Display Options" ("Ballot Design"): "Show" / "Not Show" / "Show In Help Dialog" | "Show" | "Show" | Every voter is offered "Audit ballot" on the review screen, to check that the ballot was encrypted as intended. |
| "Number of allowed votes" ("Advanced Configuration"): a whole number | 1 | as the electoral regulation requires; 1 if it allows no re-voting | Re-voting: how many times a voter may cast a vote in this election. |
| "Initialize Report Policy" ("Advanced Configuration"): "Not Required" / "Required" | "Not Required" | "Required" | Voting opens in the election only after its initialization report has been generated. The election board generates the report in front of witnesses, who check that the ballot boxes are empty. |
| "Grace Period Policy" ("Advanced Configuration"): "No grace period" / "Grace period without alert" | "No grace period" | "No grace period" | Ballots are accepted only within the election period. |
| "Grace period in seconds" ("Advanced Configuration"): a whole number | 0 | 0 | as above |
| "Allow Tally" ("Advanced Configuration"): "Allowed" / "Disallowed" / "Requires Voting Period End" | "Allowed" | "Requires Voting Period End" | Votes can be counted only after the election period has ended. |

:::warning
In "Number of allowed votes", the value 0 means **unlimited** votes, not "no
re-voting". The signed package must use 1 if the electoral regulation does
not allow re-voting. If re-voting is allowed, only the last vote of each voter
is counted; ask the board to check that the imported value is the number the regulation allows. The value cannot be changed once voting
has started on any voting channel.
:::

:::warning
With "Allow Tally" set to "Allowed", the default, a tally can be started while
voting is still open. With a grace period, ballots are accepted after the end
of the election period. With "Not Show" or "Show In Help Dialog", voters are
not offered the check that their ballot was encrypted as intended, or must
search for it.
:::

## Areas: early voting

Areas are election data: only the election board reads them, and the TOE
refuses changes to them. Whether an area allows early voting therefore comes
with the imported election data. An area with the setting "Allow Early Voting"
ticked would accept ballots before online voting opens, that is, outside the
execution phase. The import refuses a package in which an area has it ticked while
the election event's channel "Early voting" is off, which the required value of the
setting "Voting Channels Allowed" ensures
([Configuration the board must verify](#configuration-the-board-must-verify)). Tell
the election organizer that no area of a package may have it ticked.

## Tenants, plugins and the telephone channel

Where the table says that the operator tells you, and you are also the operator,
you checked it yourself in
[Verifying the Installation](../../05-reference/06-deployment_guide/10-verification.md).

| Item | Required state | Your check |
|---|---|---|
| Tenants | The customer's tenant holds every election event and every account and role for election work. The super-admin tenant serves only the installation steps of the Deployment Guide, holds no election event and has no enabled personal account. | Work only in the customer's tenant. The operator tells you that election work uses only the customer's tenant. |
| Telephone voting channel | Not configured: no telephone provider and no credentials for its endpoints, which refuse every request. Voters vote only through the voting portal. | The setting "Voting Channels Allowed" is "Online" only in the election event and in each election: the board confirms this ([Configuration the board must verify](#configuration-the-board-must-verify), [Settings of each election](#settings-of-each-election)). The operator tells you that the installation configured no telephone provider. |
| Plugins | None installed: the TOE answers every plugin request that the plugin does not exist. | The operator tells you that no plugin is installed. |
| Data correction | No account holds "Datafix account" (`datafix-account`); its data-correction routes are not used. | No role contains it and no account has it ([The roles of the evaluated configuration](04-users_and_roles.md#the-roles-of-the-evaluated-configuration)). |
| Manual verification of voters | No account holds "Manually Verify Voter" (`voter-manually-verify`); voters sign in only through the external authentication server. | No role contains it and no account has it ([The roles of the evaluated configuration](04-users_and_roles.md#the-roles-of-the-evaluated-configuration)). |

## Publishing the configuration version

The settings of this chapter, the election data and the signing rules of the
protected actions become valid for voting only as a published configuration
version that the election board has approved. The election board publishes it:
the tab "Publish" and the changes of a version show election data, so the
administrator role holds neither "Read Publish" (`publish-read`) nor "Edit
Publish" (`publish-write`). The TOE accepts a publication only in the
preparation phase and refuses it in every other phase.
To publish and approve the configuration version:

1. Check once more the settings of this chapter that you can see, and the protected actions and the
   self-test interval, timeout and additional conditions
   ([Security and Self-Tests](07-self_tests.md)).
2. Tell the election board that the settings of the election event are complete,
   once the operator has told you that the table of
   [Tenants, plugins and the telephone channel](#tenants-plugins-and-the-telephone-channel)
   holds for this election event (plugins, telephone provider and tenants), and hand
   it your copy of the
   [checklist](11-security_relevant_events.md#checklist-before-the-election-board-approves-the-configuration-version).
3. A board member selects "Generate Publication" and "Publish Changes". The
   required board members sign the signing request "Approve a configuration
   version" after comparing its "Changes in this version" with the package they
   approved.
4. The election board tells you when the version is approved. Then run the checks
   with the purpose **On request**
   ([Run the checks](07-self_tests.md#run-the-checks)) and confirm that all six
   finish within the timeout.

From the approval on, the self-tests compare the settings with it, so change nothing
on your form on the tab "Data", in the protected actions, in the self-test settings,
or in the tabs of Settings that hold tenant settings ("Attack detection", "Look &
Feel", "LANGUAGES", "LOCALIZATION", "Countries", "Integrations"). The tenant settings
and the attack-detection thresholds count for every election event of the tenant. A
change makes the check "Approved election data" or "Approved access-control
configuration" fail, which puts the election event in **Recovery required** and, in
the execution phase, stops voting. Before the execution phase such a change needs a
new configuration version that the board approves.

If an imported Data-tab setting differs from the tables, stop before the board
approves the configuration version and ask the election organizer for a corrected
signed package. The board imports it as a new election event and archives the
wrong one ([Correcting imported election data](#correcting-imported-election-data)).
Do not repair its approved ballot design by editing settings in the TOE.
Complete the separate protected-action and certificate settings before the board
publishes and approves the new event. Do not change settings after preparation.
