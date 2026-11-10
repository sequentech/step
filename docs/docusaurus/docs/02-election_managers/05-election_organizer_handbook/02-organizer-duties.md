---
id: organizer_duties
title: Duties of the Election Organizer
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The election organizer is the organisation that holds the election event. The TOE
protects the election event only if the election organizer, the election board and
the administrators carry out the measures in this chapter. They are the security
measures that the operational environment of the TOE must provide.

## Before anything is configured

### Write the electoral regulation

Set down, before the preparation phase starts, the rules the election board
applies. The TOE enforces the rules it can enforce; the board applies the
rest. The regulation states at least:

- the election event and its elections, contests and candidates, and who is entitled
  to vote;
- the election period: the start time and the end of election time of each
  election;
- the minimum number of election board members, and for each security-critical
  action the number of board members who must authorise it (the required
  authorisations). At least two are required for every security-critical
  action;
- whether re-voting is allowed and how many votes a voter may cast;
- the time frame within which an interrupted election execution may be
  continued, and the conditions under which the election board resumes an interrupted
  election execution or requests further integrity audits;
- the interval of the automatic self-tests (see
  [Self-test intervals](11-security-relevant-events.md#self-test-intervals));
- who may observe the election event and verify the result;
- how long exported election execution data is kept, who keeps it, and when it
  is deleted
  ([Exporting the election execution data](08-evaluation-and-post-processing.md#exporting-the-election-execution-data));
- whether the election dates or the election board open voting
  ([Starting the election execution](07-execution.md#starting-the-election-execution)),
  and when the board interrupts it
  ([Interrupting the election execution](07-execution.md#interrupting-the-election-execution));
- the start and the end of the lockdown period
  ([Setting the lockdown period](06-preparation.md#setting-the-lockdown-period));
- the witnesses of the initialization report
  ([The initialization report](06-preparation.md#the-initialization-report));
- the reports needed in addition to the results
  ([Checking and publishing the result](08-evaluation-and-post-processing.md#checking-and-publishing-the-result));
- what the board may read in the post-processing phase
  ([Reading the cast votes and the voters' register](08-evaluation-and-post-processing.md#reading-the-cast-votes-and-the-voters-register));
- what happens to an election whose ballot box is not sealed
  ([A ballot box is not sealed](12-modes-and-failures.md#a-ballot-box-is-not-sealed)),
  and who decides on a restore from a backup
  ([A restore from a backup](12-modes-and-failures.md#a-restore-from-a-backup));
- who keeps the system backups and the trustee-state backups and, separately,
  their passphrases;
- who writes the board's minutes, in what form, and who keeps them. The board's
  minutes are the election board's own record of the election event, outside
  the TOE; this handbook says what goes in them.

:::warning
If the regulation does not fix the required authorisations, the administrator
cannot set them. The TOE then refuses the protected actions "Import election
data", "Approve a configuration version", "Open voting", "Close voting", "Start
counting votes", "Export election execution data", "Delete election execution
data" and "Read the voters' register in post-processing", and the other protected
actions run with no authorisation at all. If it does not fix the time frame for
continuing an interrupted election execution, the board has no basis for deciding
whether to resume.
:::

### Appoint the people

To appoint the administrators, the election board and the trustees:

1. Appoint the administrators. They configure the TOE and operate it. They are
   never members of the election board.
2. Appoint the election board, before it approves the configuration version: at
   least the minimum number of members the regulation fixes, and more members
   than any protected action needs, plus a reserve for absence. A smaller board
   cannot reach the required authorisations and blocks the election event. After
   the approval, a new account or a new role assignment fails the self-test
   check "Approved access-control configuration", which stops voting; disabling
   the account of a member who leaves, or removing its roles, does not.
3. Choose two trustees among the board members. The evaluated configuration
   has two instances of the trustee server, `trustee1` and `trustee2`, as
   [Install the trustee keys](../../05-reference/06-deployment_guide/08-installing_toe.md#install-the-trustee-keys)
   describes. Each trustee holds one share of the
   election key. The threshold, the number of trustees needed to count the
   votes, is 2, so that no single trustee can decrypt. Both trustees are
   therefore needed to count the votes: plan the counting with both, and fix
   in the electoral regulation what happens if one cannot take part (see
   [A trustee leaves](03-board-roles-and-permissions.md#a-trustee-leaves)).
4. Give the administrators the list of board members and trustees, so that
   they create one personal account for each person and give it the
   role `election-board` (see
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
  voters, by a channel suited to the protection needs of the election event, so that
  only the entitled voter receives them. Tell voters never to disclose them.
- **Signing certificates.** Each board member signs security-critical actions
  with their own signing certificate on a personal security token. Issue these
  certificates from the election organizer's certification authority and hand
  each token to its holder in person. Give the administrators the issuer
  certificate and state its SHA-256 fingerprint to them in writing, through a
  channel that protects its integrity. They import it in every election event,
  the approval event and each imported one, on its tab "Signatures", area
  "Certificates", under the heading "Trusted issuers", with
  the button "Import issuer certificates", and compare the complete fingerprint that the
  admin portal shows with the value you stated. Have the administrators set, in
  the same place:
  - the check "Registering a certificate to a person" to "Only when someone who
    can register certificates registers it", so that a certificate signs only
    after an administrator has registered it to its holder. Only
    administrators hold the permission "Signatures: register certificates"
    (`signing-certificates-register`).
  - the check "Check revocation lists" on, with the check "When a list can't be
    downloaded" set to "Don't accept signatures", if your certification
    authority publishes revocation lists that the server can download. Otherwise
    the administrators withdraw a certificate in the TOE by revoking its
    registration with the button "Revoke" on that tab.

  Tell the administrators on the day a board member leaves or changes
  function, so that they revoke the person's certificate registration with
  the button "Revoke" and disable the account, removing all its roles, that same day (see
  [A person leaves, loses the role or loses their security token](03-board-roles-and-permissions.md#a-person-leaves-loses-the-role-or-loses-their-security-token)).
- **Configuration packages.** All election data reaches the TOE as signed
  configuration packages; the TOE accepts election data in no other way. For
  each package:

  1. The election organizer prepares the package with its package tool, outside
     the TOE, and signs it with its package-signing certificate. It never ticks
     the setting "Allow Early Voting" of an area: while the election event's
     setting "Voting Channels Allowed" is "Online" only, as the evaluated
     configuration requires, the TOE refuses such a package at the import. It
     gives no election a permission label: the evaluated roles do not give the
     permission "Edit Permission Label" (`permission-label-write`), so no account
     could reach that election.
  2. Before a board member creates the first election event, it gives the
     administrators the certification
     authority certificates of the package-signing and approval certificates and
     the required number of approvals. They set them for the tenant; the TOE then
     trusts only these certificates, and the number never changes afterwards.
  3. At least the required number of board members (as the electoral regulation
     fixes it, at least two) compare the package's approval code aloud and approve
     it in the package tool, each with their own approval certificate, which the
     election organizer's certification authority issues to them.
  4. The election organizer gives the board member who imports it the package
     file and, through a channel separate from the file, its SHA-256 digest.

  The evaluated configuration has no results website: the results portal is not
  deployed
  ([Keep the results and kiosk portals off](../../05-reference/06-deployment_guide/07-hardening.md#keep-the-results-and-kiosk-portals-off)),
  and without the permission "Edit Results Publication"
  (`publish-results-write`) nobody can configure or publish it, whatever a package
  contains. If imported data is wrong, the election organizer issues a corrected,
  signed package as a new revision: the board imports it as a new election event
  and archives the wrong one, whose data stays in the TOE until the board exports
  and deletes it (see
  [Importing the election data](06-preparation.md#importing-the-election-data)).

### Instruct the voters

Before the election period starts, give every voter, through the election
organizer's own channels:

- the Handbook for Voters (AGD-01) and the name of the election event;
- the address of the voting portal, and what the login page of the external
  authentication server looks like, so that voters can recognise a false one;
- the election period: the start time and the end of election time;
- how many times they may vote (the setting "Number of allowed votes" of each
  election), and that only the last vote they cast counts;
- that the voting portal offers the button "Audit ballot", with which they can
  check that their ballot was encrypted as they chose;
- where every tally batch's verification file and matching configuration hash
  and Tally ID will be published after the election event, and how voters or
  observers check that a ballot was counted, or that this check is not available
  for their ballot (see
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
- Provide separate encrypted media and backups for each trustee. One existing
  organizer-controlled workstation can serve them in turn using the clean
  offline sessions in [Trustee Procedures](09-trustee-procedures.md). Trustee
  servers run in the protected cluster; extra physical trustee machines are
  not required.

## After the election

- In the post-processing phase, after voting has closed and the results tally
  is complete, publish every batch's verification JSON file with its matching
  full configuration hash and Tally ID on the public channel named to voters.
  Publish the anchors independently on a second channel and make the verifier available to
  voters and observers, as
  [Publishing the verification data](10-election-verifier.md#publishing-the-verification-data)
  describes.
- Have the people whom the electoral regulation names keep the exported election
  execution data and its password (see
  [Exporting the election execution data](08-evaluation-and-post-processing.md#exporting-the-election-execution-data)).
  Have the election board delete the data at the end of the retention period (see
  [Deleting the election execution data](08-evaluation-and-post-processing.md#deleting-the-election-execution-data)).

## Measures by objective

| Objective | Measure | Who | Kind |
|---|---|---|---|
| Credentials reach only the entitled voter | Distribute voters' credentials before the election period by a channel suited to the protection needs, so that only the entitled voter receives them; tell voters never to disclose them or pass them on | Election organizer | Procedural |
| Unobserved voting | Instruct voters to vote where no one can observe them; provide the Handbook for Voters | Election organizer | Procedural |
| Voters can verify their vote | Tell voters about the button "Audit ballot" and about the check that a ballot was counted; publish every tally batch's verification data and matching configuration hash and Tally ID after the election event, with the anchors independently on a second channel | Election organizer | Procedural |
| Trustworthy, instructed personnel | Appoint trustworthy administrators and board members; train them with this handbook and the Handbook for Election Administrators before they first act; administrators do not misuse their privileges; one personal account per person, never shared | Election organizer | Personnel |
| Sessions closed when not in use | Log out with the entry "Logout" under the account icon in the header whenever you stop working; lock the screen when you step away | Board members, administrators | Procedural |
| Universal election | Check the voters' register in the package before approving it, and after the import, against the organizer's own register (number of voters and samples, see [Checking the imported data and the settings](06-preparation.md#checking-the-imported-data-and-the-settings)); distribute credentials to every eligible voter | Election board, election organizer | Procedural |
| Direct election | Voters vote themselves: instruct them never to pass on their credentials; treat a report of a passed-on credential as a security-relevant event | Election organizer, election board | Procedural |
| Equal election | Check that the setting "Number of allowed votes" is the same for every voter as the regulation fixes it and that only the last vote counts; count only when every ballot box is sealed; compare the votes counted with the ballot boxes | Election board | Procedural |
| Free election | Instruct voters on unobserved voting; announce the election period and every interruption to the voters; resume an interrupted election execution only within the regulation's time frame; record every decision in the board's minutes | Election board, election organizer | Procedural |
| Audit review | Check the electoral log at the beginning of the evaluation phase (see [Checking the audit records](08-evaluation-and-post-processing.md#checking-the-audit-records)) | Election board, or a board member it appoints | Procedural |
| External authentication server | Provide it; two factors for administrators and board members; only the TOE's users can log in | Election organizer | Procedural, connectivity |
| Protected server and network | Install and operate as the Deployment Guide describes; restrict physical access to the servers | Election organizer, operators | Physical, connectivity |
| Trusted time | Provide a time server with Network Time Security | Election organizer, operators | Connectivity |
| Protected remote endpoints | Use controlled devices; connect only to the addresses the administrators give you; stop at any certificate warning | Board members, trustees, administrators | Procedural, connectivity |
| Key shares kept secret | Controlled offline trustee sessions; separate encrypted media for each trustee; the keys file never sent by email or chat or pasted into a web page; two backups in separate locked places, kept as securely as the original; the keys file handed over to the operators only in person, for the trustee's instance of the trustee server; loss or exposure reported at once; keys destroyed after the tally and the export, as the board instructs (see [Protecting your keys file](09-trustee-procedures.md#protecting-your-keys-file) and [At the end of the election event](09-trustee-procedures.md#at-the-end-of-the-election-event)) | Trustees, election board, operators | Physical, procedural |

## Reporting a security problem

Report suspected security flaws in the voting system, and enquiries about them,
to Sequent's security contact,
[security@sequentech.io](mailto:security@sequentech.io). Any other question about
the TOE goes to
[election-support@sequentech.io](mailto:election-support@sequentech.io).

- **Encryption.** Encrypted reports are recommended. Use Sequent's
  security-contact OpenPGP key, fingerprint
  `DB6B035E872AB5DA90C7194A60D3CBE30BAC91ED`. It is the key that
  [Verifying the Delivery](../../05-reference/08-delivery_verification.md)
  uses, and it is also published on Sequent's
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
  a voter voted. Do not try to exploit the problem further on a live election event.
- **Acknowledgement.** Sequent acknowledges a report within five business days
  and gives the reference of the flaw record. Use that reference for status
  enquiries.
- **Reports from others.** The election organizer forwards to
  security@sequentech.io every report of a suspected security flaw that it
  receives from voters or staff.
- **Advisories and corrections.** Sequent sends security advisories to the
  security contact that the election organizer registered with Sequent for the
  delivery. Name that contact before the delivery, keep it current, and pass every
  advisory on to the administrators. Corrections arrive as a new TOE release, which
  is accepted and deployed as
  [Verifying the Delivery](../../05-reference/08-delivery_verification.md)
  describes. A running TOE is never patched in place.
