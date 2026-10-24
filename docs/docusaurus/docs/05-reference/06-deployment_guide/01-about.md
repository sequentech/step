---
id: about
title: About the Deployment Guide
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

| Item | Value |
| --- | --- |
| Document ID | AGD-04 |
| Title | Deployment Guide |
| TOE reference | Sequent Voting Platform - uniWAHL Version, v10.0.0 |
| Version | v10.0.0 |

This guide holds the preparative procedures of the Sequent Voting Platform -
uniWAHL Version, v10.0.0 (the TOE). It describes how to accept the delivered
TOE, how to prepare its operational environment and how to install and
configure the TOE, so that it starts in its evaluated configuration.

There is exactly one evaluated configuration: all seven TOE parts deployed and
operated together, on the operational environment that this guide describes, with
every setting this guide makes. A deployment that leaves out a step, changes a
setting or adds a component the guide does not name is not the evaluated
configuration.

## Who this guide is for

This guide is for the person who runs the servers, as the Handbook for Election
Administrators calls this person; this guide calls them the operator. In the TOE's
role model this is
the administrator in their platform duties: installing the servers and the
cluster, the certificates, the connection to the external authentication server,
the TOE images and the initial accounts. The operator needs root access to the
two servers and works from the administration workstation.

The administrator's duties inside the platform (users and roles, tenant and
election event settings, self-tests) are in the
[Handbook for Election Administrators (AGD-02)](../../02-election_managers/04-administrator_handbook/01-about.md).
The administrator never imports, reads or changes election data and never
reads the TOE's audit records (the electoral log): the election board imports
the election data and reviews the electoral log. The duties of the election
board and the trustees are in the
[Handbook for Election Organizers (AGD-03)](../../02-election_managers/05-election_organizer_handbook/01-about.md),
and the voters' guidance is the
[Handbook for Voters (AGD-01)](../../03-voters/02-voter_handbook/01-about.md).

## The TOE

| Part | What it does |
| --- | --- |
| harvest | Central backend: access control, voters' register, ballot boxes, import and export |
| windmill | Worker for the asynchronous tasks |
| keycloak | Identities, sessions and tokens; the link to the external authentication server |
| ballot-verifier | The voter's check that a ballot was cast as intended |
| election-verifier | Verifies the shuffle and decryption proofs of an election |
| braid | The trustee: key generation, mixing and threshold decryption |
| b4 | The bulletin board of the trustees |

Each part is an OCI image. The guidance documents AGD-01 to AGD-04 are
delivered with them. Everything else this guide installs is operational
environment, not TOE: the servers, the operating system and the cluster
software installed from the installation kit, the TLS endpoint and the ingress
in front of the TOE, the container runtime, the
database, the object storage, the GraphQL engine, the two web portals, the
network, the time server, the external authentication server and the web
browsers.

## Order of the procedures

Carry out the procedures in this order. Each one assumes that the previous
ones are complete and that all of their checks passed.

1. [Verifying the Delivery](../08-delivery_verification.md): accept the
   delivered TOE. This is the first procedure. Nothing is installed or
   deployed before its step 7, the confirmation of receipt, is complete.
2. [Operational Environment](02-operational_environment.md): provide what the
   TOE needs around it.
3. [Installing the Servers](03-installation_kit.md): accept the installation
   kit and install the two servers.
4. [Network, Time and Cluster Checks](04-hosts_network_time.md).
5. [Certificates and TLS](05-certificates_tls.md).
6. [Connecting the Authentication Server](06-authentication_server.md).
7. [Configuring the Deployment](07-hardening.md).
8. [Installing the TOE](08-installing_toe.md), including the trustee keys and
   the trustee workstations.
9. [Initial Accounts](09-accounts.md).
10. [Verifying the Installation](10-verification.md).
11. [Self-Test and Handover](11-selftest_handover.md).

## Conventions

- Commands are for bash and run as root. Each command block says where it runs:
  on C1 (the cluster server), on C2 (the second node), on the administration
  workstation or on a trustee workstation.
- Words in angle brackets inside commands, such as `<voting-portal-host>`, are
  values you supply. Replace them, brackets included.
- The commands use the shell variables that
  [Installing the Servers](03-installation_kit.md#prepare-the-deployment-directory)
  defines in the file `/root/deploy/hosts.env`: PORTAL_HOST, ADMIN_HOST,
  VERIFIER_HOST, AUTH_HOST, IDP_ALIAS, C1, C2, R and, once
  [the customer's tenant](08-installing_toe.md#create-the-customers-tenant)
  exists, CR. Load the file in every
  new shell with `. /root/deploy/hosts.env`.
- Every step ends with a check. Do not go on until the check gives the result
  stated. If a check fails, stop, do not put the TOE into operation, and
  report it to [security@sequentech.io](mailto:security@sequentech.io), as the
  section "Reporting a security problem" of the
  [Handbook for Election Administrators (AGD-02)](../../02-election_managers/04-administrator_handbook/01-about.md)
  describes.

:::warning
Steps marked like this one are security relevant. Skipping one, or changing a
value it sets, leaves the TOE outside its evaluated configuration.
:::

- Never copy, print or send the value of a password, key or secret. The
  commands generate them into root-only files and pass them on without showing
  them.

## Document history

| Release | Changes |
| --- | --- |
| v10.0.0 | First edition for the evaluated configuration. |
