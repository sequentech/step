---
id: authentication_and_sessions
title: Authentication and Session Settings
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## How people authenticate

The TOE holds no credentials of any person. Administrators, board members,
trustees and voters all authenticate at the external authentication server of
the organisation:

- the login page of the TOE sends every person on to the external
  authentication server, which is the only identity provider of the tenant and
  of every election event;
- the TOE checks the assertion that the external authentication server returns
  (its signature, issuer, audience and validity) and binds it to the existing
  account with the same identity;
- the TOE never creates an account at a first login: only accounts that you
  created in the tenant, and voters in the imported voters' register, can sign
  in;
- the TOE offers no password, one-time-code or sign-in-code form of its own,
  no self-registration, no password reset, no "remember me" and no account
  console.

The Deployment Guide sets this up for the tenant and for the template from
which the TOE creates the realm of each election event: the external
authentication server as the only identity provider, the first-login flow that
only links to existing accounts, the admin portal and voting portal clients
with exact redirect addresses and without direct password grants, the session
limits, the brute-force detection and the recording of authentication events.
These settings have no form in the admin portal: never change them.

## What you check

Check after the deployment, and for each new election event before the
election board approves its configuration version:

1. Open the admin portal in a fresh browser window. The login page sends you on
   to the external authentication server without offering a password field, a
   one-time-code field, a link to register, a link to reset a password or a
   "remember me" option.
2. Open the voting portal address of the election event, as voters will. The
   login page behaves in the same way, and offers voters no "Contact method"
   for a sign-in code.
3. Sign in with your own account and check in the header that the account
   icon's menu offers "Logout" and no link to an account console.
4. Ask a person without an account to try to sign in with their credentials at
   the external authentication server. The TOE must refuse them; it must not
   create an account.

If any check fails, the election event is not in the evaluated configuration.
Do not let the board approve the configuration version: report the finding to
the person who runs the servers, and follow the Deployment Guide to restore the
settings.

## Authentication failure threshold

The TOE counts, per account, the consecutive failed authentication attempts it
observes: when the external authentication server reports a failed
authentication for the account, or when an assertion fails the TOE's checks.
When an account reaches the threshold, the TOE locks it temporarily and writes
a record to the electoral log: in the tenant's log for administrators, board
members and trustees, in the election event's log for voters. The threshold
applies to the tenant and to every election event of the tenant.

1. Open Settings > "SECURITY".
2. Set "Authentication failure threshold". The value is a whole number of at
   least 1; the default is 5.
3. Save.

Keep the default of 5 unless the election organizer decides on another value:
a lower value locks accounts after fewer mistakes and helps an attacker to lock
out legitimate voters, a higher value gives a password-guessing attacker more
attempts. The external authentication server keeps its own limits as well.

A locked account and the record of the threshold being reached are
security-relevant events
([Security-Relevant Events for Administrators](11-security_relevant_events.md)).
Indications of attacks across several accounts appear in the Dashboard's
"Attack detections", which the election board watches
([Audit, Monitoring and Notifications](09-audit_and_monitoring.md#attack-detections)).

## Sessions

- The admin portal, the voting portal and the ballot verifier end a session
  after 15 minutes without interaction; the TOE does not extend a session
  without the user's interaction. The single sign-on session of the TOE ends
  after the same 15 minutes.
- "Logout" in the account icon's menu ends the session at once.
- Before a sensitive action, and before a board member's authorisation of a
  security-critical action is accepted, the admin portal sends the person to
  the external authentication server for a fresh login, and the TOE accepts
  the action only within 60 seconds after it. No password is entered in the
  TOE ([Secure Use of the Admin Portal](03-admin_portal.md)).
  An existing external single-sign-on session must not skip this fresh login.
  If the 60-second interval expires while reviewing or signing, authenticate
  again and review the same request before submitting it; a token refresh does
  not restart the interval.

## Voter authentication settings of each election event

Some settings that affect authentication are part of each election event and
come with the imported package. Give the required values to the election
organizer before package approval. Check them after every import, in the
election event's tab "Data"; if any differ, follow the corrected-package
procedure instead of changing them in the TOE
([Tenant and Election Event Settings](05-tenant_and_event_settings.md)):

| Setting | Required value |
|---|---|
| "Enrollment" ("Voter Authentication") | "Disabled" |
| "OTP" ("Voter Authentication") | "Disabled" |
| "Voter Digital Certificate Policy" | "Disabled" |
| "Show User Profile" | off |

No scheduled event "Start Enrollment Period" is created: voters do not
register themselves in the evaluated configuration.

:::warning
An imported package can also carry settings of the election event's realm,
such as its brute-force detection and the recording of authentication events.
After each import, check with the person who runs the servers, as the
Deployment Guide describes, that the realm of the new election event still has
brute-force detection on with your threshold and records its authentication
events. If not, do not let the board approve the configuration version until
the setting has been restored.
:::

## Prerequisites from the Deployment Guide

These settings are made at deployment and are not visible in the admin portal;
the protection of this page depends on them:

- the secrets of the container platform are stored encrypted;
- the bulletin board of the trustees is reachable only inside the server
  network, never from outside;
- every connection from outside uses the TLS profile of the Deployment Guide,
  and the server clock is synchronised with an authenticated time service.
