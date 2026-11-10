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
and voters, and the certificates of its web hosts. Provide everything in this
chapter before you install anything. The later chapters check most of it.

## Servers

| Server | Role | Requirements |
| --- | --- | --- |
| C1 | Cluster server: runs the cluster control plane and every part of the TOE, receives all client traffic on port 443 at its TLS endpoint and holds the data of the database, the object storage and the instances of the trustee server | x86_64, UEFI boot, at least 64 GB on the system disk, at least 16 GB of memory and 4 processor cores |
| C2 | Second cluster node: runs no part of the TOE. Software outside the TOE, such as the cluster's own services and the monitoring chart renderer, runs on either server | x86_64, UEFI boot, at least 64 GB on the system disk, at least 16 GB of memory and 4 processor cores |

Together, C1 and C2 are the e-voting server of the Security Target, the IT
systems on which the TOE is installed: everything this guide and the handbooks
require of the server applies to both. Install nothing on C1 and C2 other than the
kit and the accepted TOE images, and keep them free of malicious software
(OE.Server).

:::warning
Place both servers, their network equipment and the administration
workstation in a room to which only the operators have access on their own. An
operator accompanies anyone else who must be there, such as the trustees during
commissioning ([Install the trustee keys](08-installing_toe.md#install-the-trustee-keys)).
Anyone with physical access to a server or its disks can read or change the
election data.
:::

## Administration workstation

The operator works from one administration workstation. It runs Linux with
Python 3.10 or later, bash, the OpenSSH client, curl, OpenSSL 3, jq, GnuPG,
cryptsetup, skopeo and the
GNU coreutils, diffutils and findutils. The installation commands
`./deploy check site.json` and `./deploy install site.json` refuse a workstation
that lacks any of these. It holds the SSH key for root on both servers, the TLS
private key, the client secret of the authentication server and the SMTP password:
encrypt its disk, install nothing on it beyond these tools and a web browser, and
let only the operators log in.

- Its SSH public key is the only key in the site file's `admin.authorizedKeys`
  (see [Installing the Servers](03-installation_kit.md)).
- Its address lies in the site file's `admin.allowedSourceCidr`, the only IPv4
  range from which both servers accept SSH, and no other client device is in
  that range.
- It trusts the certificates of the TOE's web hosts (see
  [Certificates](#certificates)).

## Trustee key handling

The trustee servers run in the cluster, one `braid` instance per trustee. No
additional physical trustee servers are required.

For offline key generation and custody proofs, trustees may reuse one existing
organizer-controlled workstation in separate clean offline sessions. Each keeps
their own encrypted working medium and backups. Do not mount ordinary host data
disks or another trustee's medium; disable persistent swap and hibernation,
remove the medium after use, and shut down between trustees. An ordinary shared
login or separate accounts on a persistent shared installation is insufficient.

The device has podman and skopeo prepared before it is taken offline. Load the
accepted tool as described in
[Preparing a trustee workstation](08-installing_toe.md#preparing-a-trustee-workstation).
[Reuse one existing workstation](../../02-election_managers/05-election_organizer_handbook/09-trustee-procedures.md#reuse-one-existing-workstation)
defines the session and custody checks. A dedicated encrypted workstation is an
alternative, not a per-trustee hardware requirement.

## Network

The two servers share a network segment of their own: no device other than
the two servers and the site firewall is connected to it, because the traffic
between the servers (the cluster network and the connections between the
components) is not encrypted. Filter the traffic to them at the site firewall:

| From | To | Allowed |
| --- | --- | --- |
| Voters, administrators, board members and trustees | C1 | TCP 443 only, to the TLS endpoint |
| Administration workstation | C1 and C2 | TCP 22 |
| C1 and C2 | each other | UDP 8472 and TCP 10250; TCP 6443 from C2 to C1 |
| C1 and C2 | time server | NTP (UDP 123) and NTS key establishment (TCP 4460) |
| C1 and C2 | external authentication server | TCP 443 |
| C1 and C2 | DNS server | DNS |
| C1 and C2 | mail relay | the TCP port of the relay's `smtps://` URL ([Configure security email](01-about.md#configure-security-email)) |
| C1 | the addresses of the revocation lists of the election board members' signing certificates ([Trusted issuers and certificate checks](../../02-election_managers/04-administrator_handbook/06-signing_rules.md#trusted-issuers-and-certificate-checks)) | the HTTP or HTTPS port of each address |
| Anything else | C1 or C2 | nothing |

:::warning
No other port of either server may be reachable from client networks. Port 80
stays closed: the TOE offers no unencrypted HTTP.
:::

The election organizer's certificate authority names the addresses of its
revocation lists (the CRL distribution points) in its certificates. Obtain them
from the election organizer and allow only those hosts and ports. The TOE
downloads a list only from a public address, over HTTP or HTTPS, and does not
follow redirects. If the certificate authority publishes no list that the TOE can
download, leave that row out.

The site provides a network with the capacity and availability that the election
event needs (OE.Network).

On C1 the TLS endpoint is the only service that listens on port 443; the
ingress behind it listens only on C1's loopback interface
([Set up the TLS endpoint](05-certificates_tls.md#set-up-the-tls-endpoint)).

The host names of the TOE's three web hosts (the voting portal, the
administration portal and the ballot verifier) resolve to the address of C1.

## Time

The servers take their time from the site's time servers, given in the site
file by IP address. A time server must offer NTS (Network Time Security), so
that the servers accept only authenticated time. Electoral phases, token
lifetimes and the timestamps in the electoral log depend on it. Every TOE part
runs on C1, so all of these times come from C1's clock; C2 is synchronised in
the same way.

The time server’s NTS certificate must cover that IP address and chain to a CA
trusted by the delivered server operating system. Confirm this trust requirement
when the site kit is prepared
([Agree the site plan](03-installation_kit.md#agree-the-site-plan)), including any
private site CA. The matching kit
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
- serve TLS 1.3 with the cipher suite TLS_AES_256_GCM_SHA384, the group
  secp384r1 and the signature scheme ecdsa_secp384r1_sha384 only, with an
  ECDSA P-384 certificate;
- register the TOE as a confidential client, as
  [Connecting the Authentication Server](06-authentication_server.md)
  describes;
- give the operator the one CA certificate that its server certificate chains
  to, with an ECDSA P-384 key, through a channel that guarantees its integrity
  (in person or a signed email).

## Certificates

The TOE's three web hosts need server certificates with ECDSA P-384 keys:

- the voting portal and ballot verifier hosts, which voters reach, need a
  certificate from a certification authority that the common web browsers trust,
  so that voters never see a certificate warning;
- the administration portal host must use the same certificate: the installer
  takes one certificate and checks it against all three host names.

[Certificates and TLS](05-certificates_tls.md) explains which certificate inputs
to obtain through your site's certificate process and how to check their trust
and the required TLS profile.

## People

- Operators are trustworthy and trained in this guide. The separation of the
  administrator's and the election board's roles is set up as the
  [Handbook for Election Administrators](../../02-election_managers/04-administrator_handbook/01-about.md)
  describes.
- Close SSH sessions and sign out of the admin portal whenever you stop working
  (OE.Personnel).
- Trustees keep their keys under their own control, as the
  [Handbook for Election Organizers](../../02-election_managers/05-election_organizer_handbook/01-about.md)
  describes.
