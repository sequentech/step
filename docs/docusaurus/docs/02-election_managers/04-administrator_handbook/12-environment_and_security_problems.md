---
id: environment_and_security_problems
title: Environment Measures and Security Problems
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Measures for the operational environment

The TOE is secure only if its operational environment meets the security
objectives of the Security Target. The table gives the measures that
administrators take for each objective. The Deployment Guide describes the
measures on the servers in detail; the Handbook for Election Organizers
describes the election organizer's and the election board's measures.

| Objective | What it requires | What the administrator does |
|---|---|---|
| Personnel (OE.Personnel) | Administrators and board members are trustworthy and instructed, close their sessions when they are not working with the TOE, and administrators do not misuse the TOE's security functions | Read and follow this handbook. Log out whenever you stop working ([Secure Use of the Admin Portal](03-admin_portal.md#sessions-and-logout)). Never hold or give yourself a board permission, never sign, and use your permissions only as this handbook and the election organizer's instructions say. Keep a written record of your changes. |
| Servers (OE.Server) | The servers are free of malware, protected against unauthorised software access, and physically protected | Install and harden the servers only as the Deployment Guide describes, run the TOE only from the images of the verified TOE delivery and other software only from Sequent's installation kit, and check the image digests after each change. Keep the servers in a locked room with access only for the persons the election organizer names. Do not install other software on them. Change the operating system and the cluster software only with a new installation kit from Sequent, outside the execution phase, as the section "Operating the servers" of the Deployment Guide describes. |
| Network (OE.Network) | The servers are protected against attacks from the network by a security concept for the network connection, with sufficient quality of service and availability | Expose only the interfaces the Deployment Guide names, through the TLS endpoint. Keep the bulletin board of the trustees and the internal services unreachable from outside. Report every change of the network to the election board. |
| Remote endpoints (OE.RemoteEndpoint) | The remote endpoint lets the user verify the authenticity of the TOE | Use a managed, up-to-date workstation and a browser profile without extensions, and check the HTTPS connection of the admin portal and of the external authentication server before you sign in ([Secure Use of the Admin Portal](03-admin_portal.md#your-workstation-and-browser)). Stop at any certificate warning. Keep the TLS certificates of the TOE's addresses valid, as the Deployment Guide describes. |
| Authentication server (OE.AuthServer) | The external authentication server manages credentials securely and returns verifiable assertions over a protected channel | Agree with the organisation that runs it that administrators and board members authenticate with two factors, that it withdraws a leaver's credentials on the same day, and that it reports suspected compromises to you. Keep the TOE's connection to it as the Deployment Guide sets it. |
| Time service (OE.TimeService) | A reliable time service with Network Time Security | Configure the servers' time synchronisation as the Deployment Guide describes, and act at once when self-test 6 fails ([Self-Test Settings](07-self_tests.md)). |
| Audit (OE.Audit) | The election board checks the audit records at the beginning of the evaluation phase | Give the board the permissions for the review, hold none of them yourself, and explain every record you caused when the board asks ([Audit, Monitoring and Notifications](09-audit_and_monitoring.md#supporting-the-election-boards-audit-review)). |
| Election data (OE.ElectionData) | The authentication credentials reach the eligible voters before the election period, according to the protection needs of the election, so that only the entitled voter can vote with them | The credentials stay at the external authentication server; the TOE holds none. Only voters in the voters' register can sign in, and the voters' register reaches the TOE only in a signed configuration package that the election board imports: set the tenant's signed configuration packages before the first import ([Tenant and Election Event Settings](05-tenant_and_event_settings.md#import-of-election-data)). Never enter, read or change election data. |
| Observation (OE.Observation) | Voters take care that no one observes them while voting; the election organizer instructs them | No configuration measure. The election organizer instructs the voters, as the Handbook for Election Organizers and the Handbook for Voters describe. |
| Regulations (OE.Regulations) | The election board ensures that the "universal", "direct", "equal" and "free" principles of the electoral regulations are fulfilled | Take the required values that this handbook leaves to the electoral regulation, such as "Signatures needed", "Number of allowed votes" and the self-test interval, only from the election organizer in writing. |

Backups, their protection and their restoration are part of these measures
([Modes of Operation and Failure Handling](10-modes_of_operation.md#backups)).

## Reporting a security problem

Report suspected security flaws in the voting system, and enquiries about them,
to Sequent's security contact,
[security@sequentech.io](mailto:security@sequentech.io).

- **Encryption.** Encrypted reports are recommended. Use Sequent's
  security-contact OpenPGP key, fingerprint
  `DB6B035E872AB5DA90C7194A60D3CBE30BAC91ED`. It is the key the Deployment
  Guide uses to verify the delivery, and it is also published on Sequent's
  Vulnerability Disclosure Policy page,
  https://docs.sequentech.io/docusaurus/main/docs/technology/vulnerability-disclosure-policy.
  Unencrypted reports are accepted and handled the same way.
- **Contents of a report:**
  - the TOE reference and version as the admin portal shows it under "Version";
  - what happened and its effect;
  - the date, time and time zone;
  - the steps that lead to it;
  - how to contact you.
- **Never include** passwords, one-time codes, private keys, key shares, or how
  the voter voted, and no other personal data of voters. Do not try to exploit
  the problem further on a live election.
- **Acknowledgement.** Sequent acknowledges a report within five business days
  and gives the reference of the flaw record. Use that reference for status
  enquiries.
- **Advisories.** Sequent sends security advisories to the organisation's
  registered security contact. Make sure that the election organizer has
  registered one, and that you receive the advisories.
- **Corrections.** Corrections arrive as a new TOE release, which is accepted
  and deployed as the Deployment Guide describes. Never patch a running TOE in
  place, and never run a TOE image that is not part of a verified TOE delivery.

Also tell the election board and the election organizer of every report you
make that concerns an election event.
