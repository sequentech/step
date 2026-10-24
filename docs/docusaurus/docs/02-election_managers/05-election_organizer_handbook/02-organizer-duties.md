---
id: organizer_duties
title: Duties of the Election Organizer
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The election organizer is the group of people hosting the election. The TOE
protects the election only if the election organizer, the election board and
the administrators carry out the measures on this page. They are the security
measures that the operational environment of the TOE must provide.

## Before anything is configured

### Write the electoral regulation

Set down, before the preparation phase starts, the rules the election board
applies. The TOE enforces the rules it can enforce; the board applies the
rest. The regulation states at least:

- the election and its elections, contests and candidates, and who is entitled
  to vote;
- the election period: the start time and the end of election time of each
  election;
- the minimum number of election board members, and for each security-critical
  action the number of board members who must authorise it (the required
  authorisations). Two or more are recommended for every security-critical
  action;
- whether re-voting is allowed and how many votes a voter may cast;
- the time frame within which an interrupted election may be continued, and
  the conditions under which the election board resumes an interrupted
  election execution or orders further self-tests;
- the interval of the regular self-tests during the execution phase and the
  additional conditions for self-tests (see
  [Self-test intervals](11-security-relevant-events.md#self-test-intervals));
- who may observe the election and verify the result;
- how long exported election execution data is kept, who keeps it, and when it
  is deleted.

:::warning
If the regulation does not fix the required authorisations, the administrator
cannot set them, and the TOE refuses every security-critical action whose
required authorisations are not set. If it does not fix the time frame for
continuing an interrupted election, the board has no basis for deciding
whether to resume.
:::

### Appoint the people

1. Appoint the administrators. They configure the TOE and operate it. They are
   never members of the election board.
2. Appoint the election board: at least the minimum number of members the
   regulation fixes, and at least as many as the highest number of required
   authorisations, plus a reserve for absence. A smaller board cannot reach the
   required authorisations and blocks the election.
3. Choose two trustees among the board members. The evaluated configuration
   has two instances of the trustee server, `trustee1` and `trustee2`, as the
   Deployment Guide installs them. Each trustee holds one share of the
   election key. The threshold, the number of trustees needed to count the
   votes, is 2, so that no single trustee can decrypt. Both trustees are
   therefore needed to count the votes: plan the counting with both, and fix
   in the electoral regulation what happens if one cannot take part (see
   [If your keys file is lost or exposed](09-trustee-procedures.md#if-your-keys-file-is-lost-or-exposed)).
4. Give the administrators the list of board members and trustees, so that
   they create one personal account for each person and give it the
   election board role (see
   [Election Board Roles and Permissions](03-board-roles-and-permissions.md)).

:::warning
Never let one person hold both the administrator role and the election board
role, and never share an account. Either would let one person perform a
security-critical action alone and would make the electoral log name the wrong
person.
:::

### Provide the credentials

- **Authentication.** The TOE holds no passwords. Every person, voters
  included, authenticates at the external authentication server that the
  election organizer provides. Make sure that it identifies each person
  reliably, that administrators and board members authenticate there with two
  factors, and that only the accounts in the TOE's user lists can log in.
- **Voters' credentials.** Before the election period starts, distribute the
  voters' credentials of the external authentication server to all eligible
  voters, by a channel suited to the protection needs of the election, so that
  only the entitled voter receives them. Tell voters never to disclose them.
- **Signing certificates.** Each board member signs security-critical actions
  with their own signing certificate on a personal security token. Issue these
  certificates from the election organizer's certification authority and hand
  each token to its holder in person. Give the administrators the issuer
  certificate, which they import on the election event's Signatures tab under
  "Certificates" > "Trusted issuers" ("Import issuer certificates"). Have the
  administrators set, in the same place:
  - "Registering a certificate to a person" to "Only when someone who can
    register certificates registers it", so that a certificate signs only
    after an administrator has registered it to its holder. Only
    administrators hold "Signatures: register certificates"
    (`signing-certificates-register`).
  - "Check revocation lists" on, with "When a list can't be downloaded" set to
    "Don't accept signatures", if your certification authority publishes
    revocation lists that the server can download. Otherwise the
    administrators withdraw a certificate in the TOE by revoking its
    registration with "Revoke" on the Signatures tab.

  Tell the administrators on the day a board member leaves or changes
  function, so that they revoke the person's certificate registration with
  "Revoke" and disable the account, removing all its roles, that same day (see
  [A person leaves, loses the role or loses their security token](03-board-roles-and-permissions.md#a-person-leaves-loses-the-role-or-loses-their-security-token)).
- **Configuration packages.** All election data reaches the TOE as signed
  configuration packages; the voting system accepts election data in no
  other way. The election organizer prepares each package with its package
  tool, outside the TOE, and signs it with its package-signing certificate.
  Before the import, at least the required number of election board members
  (as the electoral regulation fixes it, at least two) approve it in the
  package tool, after comparing its approval code aloud, each with their own
  approval certificate, which the election organizer's certification
  authority issues to each board member who approves packages. Give the board
  member who imports it the package file and, through a channel separate from
  the file, its SHA-256 digest. Give the administrators the certification
  authority certificates of the package-signing and approval certificates,
  and the required number of approvals, so that they set them for the tenant
  before the first import and the TOE trusts only these certificates; the
  required number is never changed afterwards. Never enable the results
  website in a package, and never tick "Allow Early Voting" in an area, as
  the administrators also tell you in writing: such a package is refused when
  it is imported. If imported data is wrong,
  issue a corrected, signed package as a new revision: the board imports it
  as a new election event and archives the wrong one (see
  [Importing the election data](06-preparation.md#importing-the-election-data)).

### Instruct the voters

Before the election period starts, give every voter, through the election
organizer's own channels:

- the Handbook for Voters (AGD-01) and the name of the election;
- the address of the voting portal, and what the login page of the external
  authentication server looks like, so that voters can recognise a false one;
- the election period: the start time and the end of election time;
- how many times they may vote (the "Number of allowed votes" of the
  election), and that only the last vote they cast counts;
- that the voting portal offers "Audit ballot", with which they can check that
  their ballot was encrypted as they chose;
- where the verification data, the configuration hash and the Tally ID of
  each election will be published after the election, and how voters or
  observers check that a ballot was counted (see
  [Publishing the verification data](10-election-verifier.md#publishing-the-verification-data));
- how to contact the election organizer if anything looks wrong, and the
  security contact for suspected security problems (see
  [Reporting a security problem](#reporting-a-security-problem)).

Instruct voters to vote where no one can observe them, never to disclose their
credentials, and to log out when they have finished. Forward every report of a
suspected security problem that you receive from voters.

### Secure the operational environment

- Have the TOE installed and hardened as the Deployment Guide (AGD-04)
  describes, by trustworthy operators, in a physically protected server room,
  on a network protected from the Internet, with a time server that supports
  Network Time Security.
- Provide board members and trustees with devices that the election organizer
  controls: an up-to-date operating system and browser, no software from
  unknown sources, screen lock on, used by one person.
- Provide each trustee with a trustee workstation that is used for nothing else,
  kept in a locked place, and backed up as
  [Trustee Procedures](09-trustee-procedures.md) describes.

## After the election

- In the post-processing phase, publish for each election the verification
  data, its configuration hash and its Tally ID on a public channel that you
  have named to the voters, and make the election-verifier available to
  voters and observers, as
  [Publishing the verification data](10-election-verifier.md#publishing-the-verification-data)
  describes.
- Keep the exported election execution data and its password as the
  electoral regulation fixes. Take the file over only with the fingerprint of
  its signing key that the board copied into its minutes, and compare it with
  the fingerprint of the public key in the file. Have the election board
  delete the data at
  the end of the retention period (see
  [Exporting the election execution data](08-evaluation-and-post-processing.md#exporting-the-election-execution-data)).

## Measures by objective

| Objective | Measure | Who | Kind |
|---|---|---|---|
| Credentials reach only the entitled voter | Distribute voters' credentials before the election period by a channel suited to the protection needs, so that only the entitled voter receives them; tell voters never to disclose them or pass them on | Election organizer | Procedural |
| Unobserved voting | Instruct voters to vote where no one can observe them; provide the Handbook for Voters | Election organizer | Procedural |
| Voters can verify their vote | Tell voters about "Audit ballot" and about the check that a ballot was counted; publish the verification data, the configuration hash and the Tally ID after the election | Election organizer | Procedural |
| Trustworthy, instructed personnel | Appoint trustworthy administrators and board members; train them with this handbook and the Handbook for Election Administrators before they first act; administrators do not misuse their privileges; one personal account per person, never shared | Election organizer | Personnel |
| Sessions closed when not in use | Log out ("Logout" in the profile menu) whenever you stop working; lock the screen when you step away | Board members, administrators | Procedural |
| Universal election | Check the voters' register in the package before approving it, and after the import, against the organizer's own register (number of voters and samples, see [Checking the imported data and the settings](06-preparation.md#checking-the-imported-data-and-the-settings)); distribute credentials to every eligible voter | Election board, election organizer | Procedural |
| Direct election | Voters vote themselves: instruct them never to pass on their credentials; treat a report of a passed-on credential as a security-relevant event | Election organizer, election board | Procedural |
| Equal election | Check that "Number of allowed votes" is the same for every voter as the regulation fixes it and that only the last vote counts; count only when every ballot box is sealed; compare the votes counted with the ballot boxes | Election board | Procedural |
| Free election | Instruct voters on unobserved voting; announce the election period and every interruption to the voters; resume an interrupted election only within the regulation's time frame; record every decision in the board's minutes | Election board, election organizer | Procedural |
| Audit review | Check the electoral log at the beginning of the evaluation phase (see [Checking the audit records](08-evaluation-and-post-processing.md#checking-the-audit-records)) | Election board, or a person it appoints | Procedural |
| External authentication server | Provide it; two factors for administrators and board members; only the TOE's users can log in | Election organizer | Procedural, connectivity |
| Protected server and network | Install and operate as the Deployment Guide describes; restrict physical access to the servers | Election organizer, operators | Physical, connectivity |
| Trusted time | Provide a time server with Network Time Security | Election organizer, operators | Connectivity |
| Protected remote endpoints | Use controlled devices; connect only to the addresses the administrators give you; stop at any certificate warning | Board members, trustees, administrators | Procedural, connectivity |
| Key shares kept secret | Trustee workstations locked away, offline, used only for the trustee tool; the keys file never sent by email or chat or pasted into a web page; two backups in separate locked places, kept as securely as the original; the keys file handed over to the operators only in person, for the trustee's instance of the trustee server; loss or exposure reported at once; keys files destroyed at the end of the retention period (see [Protecting your keys file](09-trustee-procedures.md#protecting-your-keys-file)) | Trustees, election board, operators | Physical, procedural |

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
  - the TOE reference and version as the admin portal shows it;
  - what happened and its effect;
  - the date, time and time zone;
  - the steps that lead to it;
  - how to contact you.
- **Never include** passwords, one-time codes, private keys, key shares, or how
  a voter voted. Do not try to exploit the problem further on a live election.
- **Acknowledgement.** Sequent acknowledges a report within five business days
  and gives the reference of the flaw record. Use that reference for status
  enquiries.
- **Reports from others.** The election organizer forwards to
  security@sequentech.io every report of a suspected security flaw that it
  receives from voters or staff.
- **Advisories and corrections.** Sequent sends security advisories to the
  organisation's registered security contact. Corrections arrive as a new TOE
  release, which is accepted and deployed as the Deployment Guide describes. A
  running TOE is never patched in place.
