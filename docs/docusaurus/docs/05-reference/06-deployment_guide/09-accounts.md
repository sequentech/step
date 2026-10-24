---
id: accounts
title: Initial Accounts
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

After [Configuring the Deployment](07-hardening.md) and
[Installing the TOE](08-installing_toe.md), the TOE holds these accounts. The
customer's tenant is the realm in `CR`; the super-admin tenant is the realm
in `R`, which holds no election event:

| Account | Realm | State | Used by |
| --- | --- | --- | --- |
| The service administrator (the name in `KC_ADMIN_USER`) | master | random password, only in `secrets.env` and two Secrets; the master realm is not reachable from outside | the TOE's services only |
| The first personal administrator | customer's tenant | no password; signs in only through the external authentication server | the administrator |
| `admin` (seeded when the tenant was created) | customer's tenant | no password | nobody; deleted by the administrator |
| `trustee1` and `trustee2` | customer's tenant | random passwords, only in `secrets.env` and the trustees' Secrets | the trustee instances only |
| Accounts whose names start with `service-account-` | both tenants | client secrets random | the TOE's services only |
| The installation account, `admin`, `api-user`, `trustee1`, `trustee2` and the kit's further trustee accounts | super-admin tenant | disabled; only `trustee1` and `trustee2` have a password | nobody |

In the super-admin tenant's realm `R` (`tenant-` followed by the super-admin
tenant's identifier, as in `hosts.env`), the client `service-account` stays
enabled: harvest and windmill obtain their service tokens from it. Disabling
the accounts of that realm listed above does not affect it.

No account of either tenant can sign in with a password through the portals:
the local sign-in forms are disabled, and every sign-in goes to the external
authentication server.

:::warning
Never sign in with the service administrator or a trustee account, and never
give their passwords to a person. Never set a password on a personal account:
people authenticate only at the external authentication server, with two
factors.
:::

## The first sign-in

The first personal administrator signs in for the check
[Version](10-verification.md#version), once the checks before it on that page
have passed:

1. Open `https://ADMIN_HOST/tenant` in the browser of the administration
   workstation and check that the browser shows a valid HTTPS connection to
   ADMIN_HOST.
2. Under "Select Tenant", enter the customer's short name (the "Slug" given
   when [the customer's tenant was created](08-installing_toe.md#create-the-customers-tenant))
   in "Tenant Name" and select "Continue".
3. The browser is sent to the external authentication server. Sign in there
   with both factors.
4. The browser returns to the administration portal, signed in to the account
   added in [Create the customer's tenant](08-installing_toe.md#create-the-customers-tenant).

If the authentication server does not ask for two factors, if the browser
returns to any other host, or if the portal shows an account other than the
one added, stop and report it to
[security@sequentech.io](mailto:security@sequentech.io).

Give the administrators the customer's short name: everyone enters it under
"Select Tenant" to sign in.

## Accounts after the first sign-in

The administrator then continues with the section "Users, Roles and Role
Separation" of the
[Handbook for Election Administrators (AGD-02)](../../02-election_managers/04-administrator_handbook/01-about.md):
setting up the roles with their separation, creating the personal accounts of
the election board and the trustees, and deleting the seeded account `admin`. The realm settings of this guide have already removed from
the seeded administrator groups every permission that the administrator role
must not hold: the election board's permissions, among them "Create Election
Event" (`election-event-create`), with which the election board imports
election data, "Delete Election Event" (`election-event-delete`), "Archive
Election Event" (`election-event-archive`), the "Sign:" permissions, the
scheduled events and the areas, and every permission that reads, changes or
exports election data, election execution data or audit records. No group
holds "Edit Results Publication" (`publish-results-write`), "Datafix
account" (`datafix-account`) or "Manually Verify Voter"
(`voter-manually-verify`).

:::warning
The first personal administrator account holds the remaining permissions of
the seeded administrator group, among them the user, role and permission
label permissions, with which an account could be given any permission. Use
it only for the setup that the Handbook for Election Administrators describes, and never
give an administrator account a permission that the handbook reserves for
the election board. The administrator never imports, reads or changes
election data and never reads the electoral log.
:::
