---
id: tenant_and_event_settings
title: Tenant and Election Event Settings
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Order of work

1. Once per tenant, before the first election event is imported: set up
   signed configuration packages
   ([Signed configuration packages](#signed-configuration-packages)).
2. In the preparation phase, a member of the election board imports the
   election data of each election event as a signed configuration package
   ([Import of election data](#import-of-election-data)).
3. After each import, when the election board tells you, check every setting
   of the new election event and its elections against the tables on this
   page. The signed package must already carry the required Data-tab values;
   retain them. If one differs, use [Correcting imported election
   data](#correcting-imported-election-data) before continuing.
4. Set the protected actions
   ([Configuring Protected Actions](06-signing_rules.md)), the self-test
   settings ([Self-Test Settings](07-self_tests.md)) and check the
   authentication settings
   ([Authentication and Session Settings](08-authentication_and_sessions.md))
   of the election event.
5. Tell the election board that the settings are complete. After completing
   the key ceremony, the board publishes the configuration version and approves it
   ([Publishing the configuration version](#publishing-the-configuration-version)).

All of these steps belong to the preparation phase.

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
election event. Set it before the first import. Never set `required_approvals`
below 2: with fewer approvals, one board member could bring in election data
that no other member has checked.
:::

### The tenant configuration file

`configuration_signing` is part of the tenant's settings. You change it by
backing up the tenant configuration, editing the backup and restoring it. The
backup is a ZIP file with three members:

```
tenant_configuration-<tenant-id>.csv   the tenant record, including its settings
keycloak_admin-<tenant-id>.json        the identity configuration of the tenant
export_permissions-<tenant-id>.csv     each role with its permissions
```

The tenant record is a CSV file with a header line and one data line. The
header is:

```
id,slug,created_at,updated_at,labels,annotations,is_active,voting_channels,settings,test
```

Each value of the data line is JSON text; the `settings` value is a JSON
object. After your change, `settings` contains this member, next to the
members that are already there (shortened here):

```json
"configuration_signing": {
  "policy": "required",
  "package_roots": "-----BEGIN CERTIFICATE-----\n...\n-----END CERTIFICATE-----\n",
  "staff_roots": "-----BEGIN CERTIFICATE-----\n...\n-----END CERTIFICATE-----\n",
  "required_approvals": 2
}
```

Each PEM text is one JSON string, with its line breaks written as `\n`. Several
CA certificates are concatenated in one string.

### Set it

You need "Edit Tenant" (`tenant-write`) and "View settings" (`settings-menu`).

1. Make sure nobody changes roles or permissions until you have finished: the
   restore also sets every role's permissions to the ones in the backup.
2. Open Settings > "Backup / Restore". Under "Backup" ("Backup Tenant
   configurations") select "Backup". The admin portal downloads the ZIP file.
3. Extract the ZIP file into an empty folder on your workstation. Change only
   the tenant record file; leave the other two files untouched.
4. Put the election organizer's CA certificates into two PEM files, for example
   `package-roots.pem` and `staff-roots.pem`, and add `configuration_signing`
   to `settings`. Do not edit the CSV by hand: a spreadsheet program or a
   manual edit easily breaks its quoting. Use this script, which needs only
   Python 3:

   ```python
   # usage: python3 set-signing.py <tenant-csv> <package-roots.pem>
   #        <staff-roots.pem> <approvals>
   import csv, json, sys
   path, package_roots, staff_roots, approvals = sys.argv[1:5]
   with open(path, newline="") as f:
       header, row = list(csv.reader(f))[:2]
   i = header.index("settings")
   settings = json.loads(row[i]) or {}
   settings["configuration_signing"] = {
       "policy": "required",
       "package_roots": open(package_roots).read(),
       "staff_roots": open(staff_roots).read(),
       "required_approvals": int(approvals),
   }
   row[i] = json.dumps(settings)
   json.loads(row[i])  # stops here if the result is not valid JSON
   with open(path, "w", newline="") as f:
       csv.writer(f).writerows([header, row])
   ```

5. Pack the three files into a new ZIP file, at its top level, and compute its
   SHA-256 digest on your workstation:

   ```
   zip new-tenant-config.zip tenant_configuration-*.csv keycloak_admin-*.json \
     export_permissions-*.csv
   sha256sum new-tenant-config.zip
   ```

6. In Settings > "Backup / Restore", under "Restore" ("Restore Tenant config"),
   tick "Import Tenant Configurations", "Import Keycloak Configurations" and
   "Import Roles & Permissions Configurations"; "Restore" is available only
   when all three are ticked. Select "Restore".
7. In the drawer "Import Tenant Configurations", upload the new ZIP file and
   enter its digest in "Integrity Check (SHA-256)". Start the import. Wait for
   the specific task "Import Tenant Config" to show "SUCCESS" before making
   another backup. The success of an earlier export does not confirm that
   this import has finished.
8. Make a fresh backup (step 2), save it as `fresh-tenant-config.zip`, and run
   the check below against `new-tenant-config.zip`, the exact file you restored.
   It compares the entire intended settings object, the identity configuration
   and every role-permission row. JSON object-key order and whitespace, and
   CSV row order, can change during export; array order, scalar types, headers,
   duplicate rows and all values must stay the same. The script prints only
   "PASS" or "FAIL", without showing private backup contents.

   ```sh
   python3 - new-tenant-config.zip fresh-tenant-config.zip <<'PY'
   import csv, io, json, sys, zipfile
   from collections import Counter

   def rows(data):
       return list(csv.reader(io.StringIO(data.decode("utf-8"))))

   def canonical(data):
       return json.dumps(json.loads(data), sort_keys=True, separators=(",", ":"))

   def settings(data):
       records = rows(data)
       assert len(records) == 2
       header, row = records
       value = json.loads(row[header.index("settings")])
       assert isinstance(value, dict)
       return json.dumps(value, sort_keys=True, separators=(",", ":"))

   try:
       with zipfile.ZipFile(sys.argv[1]) as intended, zipfile.ZipFile(sys.argv[2]) as fresh:
           names = intended.namelist()
           assert len(names) == len(set(names)) == len(fresh.namelist()) == 3
           assert set(names) == set(fresh.namelist())
           prefixes = ("tenant_configuration-", "keycloak_admin-", "export_permissions-")
           assert all(sum(name.startswith(prefix) for name in names) == 1 for prefix in prefixes)
           for name in names:
               before, after = intended.read(name), fresh.read(name)
               if name.startswith("tenant_configuration-"):
                   assert settings(before) == settings(after)
               elif name.startswith("keycloak_admin-"):
                   assert canonical(before) == canonical(after)
               else:
                   left, right = rows(before), rows(after)
                   assert left and right and left[0] == right[0]
                   assert Counter(map(tuple, left[1:])) == Counter(map(tuple, right[1:]))
   except Exception:
       print("FAIL")
       sys.exit(1)
   print("PASS")
   PY
   ```

   Continue only after "PASS". If the import task fails, parts of the restore
   may already have been applied: repeat steps 6 to 8 with the same intended
   file. If the import succeeds but the check prints "FAIL", keep both files
   protected and investigate the difference before continuing or restoring
   again.

:::danger
If the `settings` value in the file you restore is not valid JSON, the TOE does
not refuse the file: it clears all the tenant's settings, which also removes
`configuration_signing`. Never skip the checks of steps 4 and 8.
:::

:::warning
The backup contains the tenant's identity configuration and its roles. Keep it
on your workstation only as long as you need it, and delete every copy when you
have finished. Make the backup and the restore in one session, so that the
restore cannot undo role changes made in between.
:::

Set `configuration_signing` once, before the first election event is imported.
`required_approvals` changes only from not set to a number, by you, in the
preparation phase, and never afterwards; do not change `policy` or
`package_roots` afterwards either. In `staff_roots`, list from the start the election
organizer's current staff CA and, if the organizer has one, its successor. A
staff CA is replaced only between elections, when no election event of the
tenant is in the preparation, execution or evaluation phase. If the election
organizer tells you that the number of approvals must change, do not change it:
tell the election board and the election organizer.

## Import of election data

A member of the election board imports the election data; you do not. The
voting system accepts election data only as a signed configuration package,
through "Import Election Event". Each import creates a new election event in
the preparation phase. The import needs "Create Election Event"
(`election-event-create`), which only the "Election board" role holds, and the
package must carry the approvals of at least `required_approvals` different
board members, made with certificates that chain to `staff_roots`. The board
members make their approvals outside the TOE, and the TOE verifies them when
the package is imported. The board's procedure, with the digest in "Integrity
Check (SHA-256)", is in the
[Handbook for Election Organizers](../05-election_organizer_handbook/01-about.md).

Your part:

1. Before the first import, set `configuration_signing`
   ([Signed configuration packages](#signed-configuration-packages)).
2. Before the package is approved, give the election organizer the required
   Data-tab values in the tables of this page. After each import, when the
   election board names the new election event, check that those values are
   present and retain them. You see the settings, not the election data.
3. If the election board reports that an import was refused although the
   package is the one the election organizer released, check with a backup
   (steps 2 and 8 of [Set it](#set-it)) that `configuration_signing` still has
   your values. Do not change it. Treat the refusal as a security-relevant event
   ([Security-Relevant Events for Administrators](11-security_relevant_events.md)).

The TOE refuses every other way in for election data: "Create an Election
Event", the "Import" of the Voters tab and of Areas, "Import Candidates",
"Import schedule", and every change of imported election data (the voters'
register, the candidate list, the display data of the ballot, the set of
election rules, the election dates and the election period end time).

Some settings available to administrators also affect the ballot design that
is approved in the signed package. The election organizer includes the required
event and election Data-tab values before the package approvals. Administrators
verify those imported values; they do not change them to repair a package.
Otherwise publication can refuse the changed ballot design. The tenant settings,
protected-action rules, certificate registrations and self-test settings are
configured separately as their procedures describe. Areas remain election data.

### Correcting imported election data

If the imported election data is wrong, nobody corrects it in the TOE:

1. The election organizer issues a corrected signed configuration package.
2. A member of the election board imports it. It becomes a new election event.
3. The election board archives the wrong election event in the preparation
   phase with "Archive this Election Event". An archived election event never
   opens voting; its data and its audit records stay. It is never unarchived.
4. Check the settings of the new election event as after every import.

:::danger
Never delete an election event, and never ask for one to be deleted. Deleting
an election event also deletes its audit records. Only the election board
deletes election events, after the export of the election execution data and
with the required authorisations; the TOE refuses every other deletion.
:::

## Settings of the election event

Open the election event and its tab "Data". The settings are grouped in
sections; the table names the section of each.

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
  started ("This cannot be changed once voting has started."). The required
  value must already be in the signed package; a different imported value
  requires a corrected package and a new election event.
- Check "Checks available until" in the time zone that the field shows.
  It must match the end of the election period as the election organizer
  states it to you in writing, with its time zone.
- "Show User Profile", "Enrollment" and "OTP" count as on or "Enabled" when
  their values are not set. The signed package must contain explicit off or
  "Disabled" values. Reopen the imported event to check them; do not toggle
  these controls to repair an incomplete package.
- "Results Website" is not one of your settings: the admin portal shows it
  only to accounts with "Edit Results Publication" (`publish-results-write`),
  which no role holds, and the TOE refuses the import of a package that enables
  the results website. "Publish to results website" is never available.
- An imported package can carry any value of these settings. Check all of them
  after every import, even when the previous election event was correct.

:::warning
Never select "Allow Automatic Ceremonies". It makes the switch "Automatic
Ceremony" available when a key ceremony is created, and an automatic ceremony
removes the trustees' custody confirmation and their authorisation of the
tally: the votes could then be decrypted without the trustees' consent. The
evaluated configuration forbids automatic ceremonies. The switch "Automatic
Ceremony" of each key ceremony decides whether the trustees confirm custody,
whatever this policy says. The election board, which creates
the key ceremony ("Create Key Ceremony" in the Keys tab), never switches on
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
No results website is part of the evaluated configuration, and the TOE refuses
the import of a package that enables one. Never give any role "Edit Results
Publication" (`publish-results-write`), so that "Publish to results website" is
never available.
:::

## Settings of each election

Open each election of the election event and its tab "Data".

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
is counted; check that the imported value is the number the regulation allows. The value cannot be changed once voting
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
with the imported election data. An area with "Allow Early Voting" ticked
would accept ballots before online voting opens, that is, outside the
execution phase, so the import refuses a package in which an area has it
ticked. Tell the election organizer in writing that no area of a package may
have it ticked.

## Tenants, plugins and the telephone channel

| Item | Required state | Your check |
|---|---|---|
| Tenants | The customer's tenant holds every election event and every account and role for election work. The super-admin tenant serves only the installation steps of the Deployment Guide, holds no election event and has no enabled personal account. | Work only in the customer's tenant. The person who runs the servers confirms in writing that election work uses only the customer's tenant. |
| Telephone voting channel | Not configured: no telephone provider and no credentials for its endpoints, which refuse every request. Voters vote only through the voting portal. | "Voting Channels Allowed" is "Online" only in the election event and in each election. The person who runs the servers confirms in writing that the installation configured no telephone provider. |
| Plugins | None installed: the TOE answers every plugin request that the plugin does not exist. | The person who runs the servers confirms in writing that no plugin is installed. |
| Data correction | No account holds "Datafix account" (`datafix-account`); its data-correction routes are not used. | No role contains it and no account has it ([Users, Roles and Role Separation](04-users_and_roles.md#permissions-on-election-data-and-audit-records)). |
| Manual verification of voters | No account holds "Manually Verify Voter" (`voter-manually-verify`); voters sign in only through the external authentication server. | No role contains it and no account has it ([Users, Roles and Role Separation](04-users_and_roles.md#permissions-on-election-data-and-audit-records)). |

## Publishing the configuration version

The settings of this page, the election data and the signing rules of the
protected actions become valid for voting only as a published configuration
version that the election board has approved. The election board publishes it:
the tab "Publish" and the changes of a version show election data, so the
administrator role holds neither "Read Publish" (`publish-read`) nor "Edit
Publish" (`publish-write`). The TOE accepts a publication only in the
preparation phase and refuses it in every other phase.

1. Check the settings of this page once more, and the protected actions and the
   self-test settings.
2. Tell the election board in writing that the settings of the election event
   are complete, once you hold the written confirmation of the person who runs
   the servers for this election event (plugins, telephone provider and
   tenants, as in the table above).
3. A board member selects "Generate Publication" and "Publish Changes". The
   required board members sign the signing request "Approve a configuration
   version" after comparing its "Changes in this version" with the package they
   approved.
4. The election board tells you when the version is approved. You know that it
   worked when the board confirms the approval; from then on change no setting
   without telling the board.

If an imported Data-tab setting differs from the tables, stop before the board
approves the configuration version and ask the election organizer for a corrected
signed package. The board imports it as a new election event and archives the
wrong one ([Correcting imported election data](#correcting-imported-election-data)).
Do not repair its approved ballot design by editing settings in the TOE.
Complete the separate protected-action and certificate settings before the board
publishes and approves the new event. Do not change settings after preparation.
