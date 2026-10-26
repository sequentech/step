---
id: operational_environment
title: Operational Environment
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The TOE relies on its operational environment for physical protection,
network filtering, time, the authentication of administrators, board members
and voters, and the certificates of its web hosts. Provide everything on this
page before you install anything. The later pages check most of it.

## Servers

| Server | Role | Requirements |
| --- | --- | --- |
| C1 | Cluster server: runs the cluster control plane, receives all client traffic on port 443 at its TLS endpoint and holds the data of the database, the object storage and the instances of the trustee server | x86_64, UEFI boot, at least 64 GB on the system disk, at least 16 GB of memory and 4 processor cores |
| C2 | Second cluster node: runs workloads | x86_64, UEFI boot, at least 64 GB on the system disk, at least 16 GB of memory and 4 processor cores |

Together, C1 and C2 are the e-voting server of the Security Target, the IT
systems on which the TOE is installed: everything this guide and the handbooks
require of the server applies to both.

:::warning
Place both servers, their network equipment and the administration
workstation in a room to which only the operators have access. Anyone with
physical access to a server or its disks can read or change the election data.
:::

## Administration workstation

The operator works from one administration workstation. It runs Linux with
Python 3.10 or later, bash, the OpenSSH client, curl, OpenSSL 3, jq, GnuPG,
cryptsetup, skopeo and the
GNU coreutils, diffutils and findutils.

- Its SSH public key is the only key in the site file's `admin.authorizedKeys`
  (see [Installing the Servers](03-installation_kit.md)).
- Its address lies in the site file's `admin.allowedSourceCidr`, and no
  other client device is in that range.
- It trusts the certificates of the TOE's web hosts (see
  [Certificates](#certificates)).

## Trustee workstations

Each trustee has their own workstation, used only for the trustee tool:

- it has no network connection while it holds trustee keys;
- its disk is encrypted, and only the trustee can unlock it;
- podman and skopeo are installed on it before it is taken offline;
- files move to and from it only on removable media that the trustee keeps.

[Installing the TOE](08-installing_toe.md#preparing-a-trustee-workstation)
loads the trustee tool onto it.

## Network

The two servers share a network segment of their own: no device other than
the two servers and the site firewall is connected to it, because the traffic
between the servers (the cluster network and the connections between the
components) is not encrypted. Filter the traffic to them at the site firewall:

| From | To | Allowed |
| --- | --- | --- |
| Voters, administrators and board members | C1 | TCP 443 only, to the TLS endpoint |
| Administration workstation | C1 and C2 | TCP 22 |
| C1 and C2 | each other | UDP 8472 and TCP 10250; TCP 6443 from C2 to C1 |
| C1 and C2 | time server | NTP (UDP 123) and NTS key establishment (TCP 4460) |
| C1 and C2 | external authentication server | TCP 443 |
| C1 and C2 | DNS server | DNS |
| Anything else | C1 or C2 | nothing |

:::warning
No other port of either server may be reachable from client networks. Port 80
stays closed: the TOE offers no unencrypted HTTP.
:::

On C1 the TLS endpoint is the only service that listens on port 443; the
ingress behind it listens only on C1's loopback interface
([Set up the TLS endpoint](05-certificates_tls.md#set-up-the-tls-endpoint)).

The host names of the TOE's three web hosts (the voting portal, the
administration portal and the ballot verifier) resolve to the address of C1.

## Time

The servers take their time from the site's time servers, given in the site
file by IP address. A time server must offer NTS (Network Time Security), so
that the servers accept only authenticated time. Election phases, token
lifetimes and the timestamps in the electoral log depend on it.

The time server’s NTS certificate must cover that IP address and chain to a CA
trusted by the delivered server operating system. Confirm this trust requirement
when the site kit is prepared, including any private site CA. The matching kit
must configure every time source with NTS; plain NTP is not an acceptable
fallback. If the authenticated-time check fails, obtain a corrected kit or time
service rather than disabling certificate verification or editing the installed
time configuration.

## External authentication server

Administrators, election board members and voters authenticate at an external
authentication server; the TOE holds no passwords for them. The server must:

- be an OpenID Connect provider that signs its ID tokens and publishes its
  signing keys;
- authenticate administrators and election board members with two factors;
- serve TLS 1.3 with the cipher suite TLS_AES_256_GCM_SHA384 and the group
  secp384r1 only, with an ECDSA P-384 certificate;
- register the TOE as a confidential client, as
  [Connecting the Authentication Server](06-authentication_server.md)
  describes;
- give the operator the CA certificate that its server certificate chains to,
  through a channel that guarantees its integrity.

## Certificates

The TOE's three web hosts need server certificates with ECDSA P-384 keys:

- the voting portal and ballot verifier hosts, which voters reach, need a
  certificate from a certification authority that the common web browsers trust,
  so that voters never see a certificate warning;
- the administration portal host can use the same certificate.

[Certificates and TLS](05-certificates_tls.md) creates the key and the request.

## People

- Operators are trustworthy and trained in this guide. The separation of the
  administrator's and the election board's roles is set up as the
  [Handbook for Election Administrators (AGD-02)](../../02-election_managers/04-administrator_handbook/01-about.md)
  describes.
- Trustees keep their keys under their own control, as the
  [Handbook for Election Organizers (AGD-03)](../../02-election_managers/05-election_organizer_handbook/01-about.md)
  describes.
