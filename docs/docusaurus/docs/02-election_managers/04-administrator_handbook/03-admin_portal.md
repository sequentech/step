---
id: admin_portal
title: Secure Use of the Admin Portal
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Interfaces available to the administrator

| Interface | What you use it for |
|---|---|
| Admin portal, in your browser over HTTPS | every configuration and monitoring task of this handbook |
| Login page of the TOE, which sends you on to the external authentication server | signing in and confirming sensitive actions |
| The server procedures of the Deployment Guide, used by the person who runs the servers | deployment, hardening, operation, backups and the image check |

There is no other interface for administrators in the evaluated configuration.
Over HTTPS, the identity and access management component offers only its
sign-in services: its administration console is not routed, so it cannot be
reached from outside the servers, and its account console is switched off. Use
the TOE's programming interfaces only through the admin portal.

:::warning
Do not use scripts, command-line clients or other tools against the TOE's
interfaces in place of the admin portal. Their requests are subject to the same
server-side checks, but they bypass the confirmation dialogs of the admin
portal, and this handbook gives no secure way to use them.
:::

## Your workstation and browser

- Use a workstation that your organisation manages and keeps up to date, with
  an up-to-date browser. Do not use shared or public computers.
- Use a browser profile without extensions for the admin portal.
- Do not let the browser save the credentials of the external authentication
  server.
- Type the admin portal's address, or use a bookmark you made from the address
  that the Deployment Guide set up. Do not follow links in emails.
- Check that the browser shows a valid HTTPS connection for the admin portal
  and for the external authentication server before you sign in. If the
  browser reports a certificate problem, do not continue: report it to the
  person who runs the servers.

## Signing in

To sign in to the admin portal:

1. Open the admin portal at its address followed by "`/tenant`". For example,
   if its address is `https://admin.example.org`, open
   `https://admin.example.org/tenant`. Under "Select Tenant", enter your
   organisation's tenant name in "Tenant Name" and select "Continue". After five failed attempts the admin portal shows "Too many attempts. Please try
   again later."
2. The admin portal sends you to the login page of the TOE, which sends you to
   the external authentication server of your organisation.
3. Authenticate there with your two factors.
4. The external authentication server sends you back to the admin portal,
   signed in with your personal account.

The TOE holds no password for you: it accepts only the assertion of the
external authentication server, after checking it. If you cannot sign in,
contact the organisation that runs the external authentication server; no
administrator can reset your credentials in the TOE.

:::warning
If the TOE ever shows you a password or one-time-code form of its own, or
offers to register a new account, stop and report it: the evaluated
configuration has none ([Authentication and Session Settings](08-authentication_and_sessions.md)).
:::

## Confirming sensitive actions

Before some sensitive actions the admin portal shows the dialog "Confirm
Action", which names the action and its consequence. Read it, then select
"Confirm" or "Cancel".

Where an action needs a fresh login, you authenticate again after "Confirm": the admin portal sends you to the
external authentication server of your organisation for a fresh login, and you
authenticate there with your two factors. You enter no password in the TOE.
The external authentication server sends you back to the admin portal, and the
TOE carries out the action only if this fresh login is no more than 60 seconds
old; otherwise it refuses the request, and you start the action again.

Board members authenticate again in the same way before the TOE accepts their
authorisation of a security-critical action.

## Sessions and logout

- After 15 minutes without interaction the TOE ends your session in the admin
  portal. Sign in again to continue.
- To end your session at once, open the account icon in the header, select
  "Logout" and confirm "Are you sure you want to logout?" with "OK".
- Log out whenever you stop working with the TOE, even for a short break, and
  lock your workstation when you leave it.
- If the admin portal shows the dialog "Warning" with "Do you want to stay
  connected to this tenant or logout?", select "Logout" unless you are
  continuing your own work in the same tenant.

:::warning
An open session in an unattended browser lets anyone at the workstation act
with your permissions. The Security Target requires administrators to close
their sessions while they are not actively working with the TOE.
:::

## Checking the version of the TOE

The header of the admin portal shows "Version:" followed by the software
release. In a wide window it also shows "Hash:" followed by the first eight
characters of the source revision of the TOE, in the form `1A2B-3C4D`.

Check after the deployment, after every update and at the start of each
election event's preparation phase that the version is exactly `10.0.0`.

The header alone does not prove which images are running. The person who runs the
servers checks the running images against the verified TOE delivery, as the
Deployment Guide describes, and confirms the result to you in writing after
the installation and for each new election event.

:::warning
If the version differs, or the person who runs the servers reports that the
image check failed, the system is not the evaluated TOE. Do not start or
continue an election event on it. Inform the election board and the election
organizer, and report it as a security problem
([Environment Measures and Security Problems](12-environment_and_security_problems.md#reporting-a-security-problem)).
:::
