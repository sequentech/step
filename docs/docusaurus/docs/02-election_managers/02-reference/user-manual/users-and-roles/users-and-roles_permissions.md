---
id: users-and-roles_permissions
title: Permissions
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Permissions are assigned to roles under **Users and Roles** > **Roles**. A user receives the
permissions of the roles assigned to them. Use the narrowest role that covers the user's work;
having access to an election event does not automatically grant access to every action or field in
it.

## Secret Voter Field Permissions

Secret voter fields have independent read and write permissions so an operator can manage a value
without necessarily being able to see it.

| Permission | Allows |
|---|---|
| `voter-secret-attribute-read` | Explicitly reveal a secret value, include decrypted secret fields in a voter export, use declared secret variables in voter-level communications or reports, and download restricted decrypted voter exports. |
| `voter-secret-attribute-write` | Set, replace, clear, or import a secret voter value. It does not allow revealing an existing value. |

These permissions supplement the ordinary permission for the operation:

| Operation | Required permissions |
|---|---|
| Reveal a value | Voter read access and `voter-secret-attribute-read` |
| Create or edit a secret value | The corresponding voter create/write access and `voter-secret-attribute-write` |
| Import a CSV containing a secret column | Voter import/create access and `voter-secret-attribute-write` |
| Export decrypted secret columns | Normal voter export access and `voter-secret-attribute-read` |
| Send or generate an output that declares secret fields | The normal communication/report permission and `voter-secret-attribute-read` |

A standard voter list or export never needs either secret permission because secret columns are
omitted. A decrypted voter-export document remains restricted: downloading it checks
`voter-secret-attribute-read` again.

Assign read and write independently where duties require it. For example, an import operator can
receive secret-write without secret-read, while a support operator who must inspect a value can
receive secret-read without permission to change it.

## Monitoring Dashboard Permissions

Once an election event is set up for monitoring, its **Dashboard** tab shows the event's monitoring
dashboards, and each election's **Dashboard** tab shows them for that election's Post. Two
permissions govern them:

| Permission | Allows |
|---|---|
| `monitoring-view` | See the monitoring dashboards, view a widget's data and export it. |
| `monitoring-configure` | Edit widgets, dashboards and themes, and reset the event to a preset. |

The tab itself still needs `admin-dashboard-view` on the event, or `election-dashboard-tab` on an
election. With the tab but without `monitoring-view`, a user sees the standard dashboard. Permission
labels still apply: a user whose roles carry labels sees only the elections those labels allow, and
cannot choose any other.

Configuring changes the event, so it also needs `election-event-write`, and it is not possible once
the event is locked down.

New tenant realms come with both permissions: the default tenant realm template and the COMELEC
template grant both to the `admin` group and only `monitoring-view` to `admin-light`. A deployment
that stores the tenant template in S3 (`KEYCLOAK_TENANT_REALM_CONFIG_S3_KEY`) must upload the updated
template before new tenants get them. Existing realms are not changed; grant the permissions to the
intended roles by hand.

See [Monitoring](../../02-election-event/02-election_management_election-event_monitoring.md) for what
each permission shows and allows.

## Signature Permissions

The **Signatures** tab of an election event decides which protected actions need the signatures of
several people. Each part of it has its own permission, and signing each action has another.

| Permission | Allows |
|---|---|
| `election-event-signatures-tab` | Show the Signatures tab. It also needs at least one read permission below. |
| `signing-rules-read` | See the Protected actions sub-tab. |
| `signing-rules-write` | Edit the protected actions' rules. |
| `signing-certificates-read` | See the Certificates sub-tab. |
| `signing-issuers-write` | Import and remove trusted issuers. |
| `signing-checks-write` | Change the certificate checks. |
| `signing-certificates-register` | Register a certificate to a person. |
| `signing-certificates-revoke` | Revoke a registered certificate. |
| `signing-requests-read` | See the Requests sub-tab. Users limited by permission labels see their Posts' requests. |
| `signing-requests-cancel` | Cancel someone else's waiting request. The person who started a request can always cancel it. |
| `signing-requests-export` | Export the requests as CSV. |
| `sign-<action>` | Sign that action: `sign-initialize-voting`, `sign-open-voting`, `sign-close-voting`, `sign-generate-election-returns`, `sign-generate-reports`, `sign-transmit-results`, `sign-approve-voter`, `sign-approve-configuration`, `sign-key-ceremony` and `sign-tally-key`. |

Changing who can sign an action takes `role-write`, from the rule drawer or here. With `role-write`
but without `user-permission-write`, a user can turn only the `sign-<action>` permissions on or off; the
other permissions in the role's grid stay read-only for them. Changing any other permission still takes
both. Signers need no permission on the tab.

Turning a `sign-<action>` permission on or off for a role, creating a role that has one, and deleting a
role that holds one are written to the Logs of each election event that has a rule for that action. A
change that changes nothing (setting a permission the role already has) is not written. Some changes to
who can sign are not written to the Logs yet (a known gap):

- adding a user to a role, or removing them;
- granting a `sign-<action>` permission through a subgroup or a composite role. People who hold it that
  way can still sign, like those whose role holds it directly.

The platform's own permissions, these included, can't be deleted from **Users and Roles**.

New tenant realms get all of these permissions, held by the `admin` group. The janitor's client
template also has a sample preset: a **Configuration Manager** group (`configuration-manager`) with the
Protected actions, a **Security Officer** group (`security-officer`) with the Certificates, an **OFOV**
group (`ofov`) with the Requests, and an **Auditor** group (`auditor`) that reads all three. SBEIs can
sign the voting, results and voter approval actions, and trustees their key steps. The four groups also
get `election-event-read`, to reach an election event, and `election-read` and `area-read`, so that the tab
names Posts and countries; they don't get the general `admin-user` role. The Configuration Manager also gets
`role-read`, for the list of roles the rule editor picks signers from. So that the Security Officer can
change who signs, the preset also gives it `users-menu`, `role-read`,
`user-permission-read` and `role-write`, which open Users and Roles and change only the sign
permissions. It also gets `user-read`, because registering a certificate picks the person from the
tenant's users. The preset is a sample, still to be confirmed; adjust it to the organization's own
roles.

Existing tenant realms get the permissions, with their labels, when Windmill's scheduler (beat) starts
after an upgrade, retrying for about 15 minutes if Keycloak isn't reachable yet;
`step-cli step migrate-realm-permissions` does the same by hand. Only the permissions
are added: no role receives them. Assign them to roles in **Users and Roles** > **Roles**.

[Signature permissions](../../02-election-event/16-signatures/06-election_management_election-event_signatures_permissions.md)
lists each of these permissions with its label in Users and Roles, and the sample preset's
groups and rules; [Signatures](../../02-election-event/16-signatures/01-election_management_election-event_signatures.md)
describes the tab.
