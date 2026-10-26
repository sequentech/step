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
| Document version | 1.0 |
| Software release | v10.0.0 |

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

## Use the bundled deployment commands

After accepting the delivered guidance package under **Verifying the Delivery**,
use its sibling **deployment** folder. It contains the complete reviewed command
sources and their hashes; no repository checkout or online download is needed.
The separate installation kit is checked in section 3 before installing servers.

Copy the whole folder to each machine that will execute a step. Use a root shell
and enter that folder. The workstation needs Python 3.10 or later, Bash, and the
operational tools named in this guide; the corresponding installation kit supplies
Python 3 and Bash on C1 and C2. Check these prerequisites before starting.

On the administration workstation, with a root terminal already open in the
delivered `deployment` folder, enter these commands **one at a time**:

```bash
./deploy check
```

Expected: `Package hashes match. Independently verify its delivery signature and source identity.`
This detects changed package files; the delivery signature and independent source
verification establish their authenticity. If the package check stops, obtain the
verified complete folder; do not change its checksums to suppress the error.

```bash
./deploy configure admin
```

Enter the seven public site values and the absolute directory containing the
installation kit to be checked. Expected: `Public configuration saved. No passwords,
keys or client secrets belong in operator.json.` Passwords, keys and client secrets
never belong in this form. If an existing configuration differs, inspect and
reconcile the machine's approved site values before continuing; do not overwrite
them blindly.

```bash
./deploy session admin
```

Expected: a new interactive prompt identifying the `admin` deployment session.
If the machine-identity or role check stops, confirm that you configured this
workstation for this role; never copy another machine's `operator.json` to bypass it.

Repeat configure and session on each other machine with its role below, using the
same public site values. Each machine has its own root-only public configuration
in `/root/deploy/operator.json`; `hosts.env` is loaded automatically in every session.
An existing different configuration is refused instead of overwritten. The server
address and machine identity are checked before a command runs.

| Session | Where to start it |
| --- | --- |
| `admin` | Administration workstation, initially in the installation kit directory |
| `c1` | Root shell on C1, initially in `/root/deploy` |
| `c2` | Root shell on C2, initially in `/root/deploy` |
| `client` | Separate machine in the client network, for reachability checks |
| `trustee` | Offline trustee workstation under the trustee's physical custody |

For example, in a root terminal in the copied `deployment` folder **on C1**, enter
these separately:

```bash
./deploy configure c1
./deploy session c1
```

The launcher never changes machines or opens an SSH session automatically.
A session label does not replace verifying network position or physical custody.

`step`, `show` and `steps` are commands provided **inside this deployment session**;
they are not installed as ordinary system commands. If your terminal says
`step: command not found`, open the appropriate `./deploy session ROLE` from the
delivered folder first (replace ROLE with the session name in the table).

Inside a session, enter `steps` to list the available named commands. Each procedure
below provides the exact command to run, its expected result and the response to an
unexpected result. For example, when you reach the cluster-secret encryption check
on C1, the command is:

```bash
step 04-check-secret-encryption
```

It must include `Encryption Status: Enabled` in its output. This is a readable name
for that single check, not an instruction to execute all of chapter 4. The old
numeric names remain compatibility aliases; use the descriptive names in this guide.

To inspect the implementation without executing it, the optional command is:

```bash
show 04-check-secret-encryption
```

Long implementations are in `deployment/steps/`, outside the main procedure text.
The reference screenshots show the command preview and documented acceptance
checks; they are examples for comparison, not evidence that your installation has
passed. Site addresses, host names, timestamps and table layout can vary; the stated
security conditions must still match. Read the prerequisites before running a step. Public
values needed later are prompted when used. Existing private password prompts
and private input files remain unchanged; command output is not recorded.

Keep the session open: variables, helper functions and working-directory changes
are needed by later steps. Use `exit` when changing machine or finishing. After a
failure, inspect the current state and the stated check; never replay a write
blindly. A successful command exit is **not** acceptance of the step. Compare
its actual output with every condition in this guide before continuing. Do not
record these sessions with shell tracing or terminal transcripts.

## Conventions

- Commands are for bash and run as root. Each command block says where it runs:
  on C1 (the cluster server), on C2 (the second node), on the administration
  workstation or on a trustee workstation.
- Words in angle brackets in the bundled source identify public inputs. The
  launcher prompts for these values; do not edit the verified command package.
- The commands use the shell variables that
  [Installing the Servers](03-installation_kit.md#prepare-the-deployment-directory)
  defines in the file `/root/deploy/hosts.env`: PORTAL_HOST, ADMIN_HOST,
  VERIFIER_HOST, AUTH_HOST, IDP_ALIAS, C1, C2, R and, once
  [the customer's tenant](08-installing_toe.md#create-the-customers-tenant)
  exists, CR. The launcher loads this file in every new session; outside the
  launcher, load it with `. /root/deploy/hosts.env`.
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
