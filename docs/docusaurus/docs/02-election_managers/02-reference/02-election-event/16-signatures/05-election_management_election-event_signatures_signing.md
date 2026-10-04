---
id: election_management_election_event_signatures_signing
title: Signing a protected action
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This page is for the people who sign protected actions: a Post's board members, officials
who approve voters or configuration versions, and trustees. Every action is signed the
same way, with the request panel and the signing dialog.

## Before you start

You need:

- your own account in the Admin Portal, with the permission to sign the action (shown in
  **Users and Roles** as "Sign: *action*") and access to your Post;
- your **security token**, the USB drive with your certificate file (`.p12` or `.pfx`),
  and the file's password;
- a certificate from an issuer the election event trusts. Depending on the event, it is
  registered to you the first time you sign with it, or an officer registers it for you
  beforehand.

Signing happens in your browser. Your certificate file, its private key and its password
are never sent: only your signature and your public certificate go to the server. The
dialog repeats this on every step.

## Starting a protected action

Start the action where you always do, for example **Start voting** in a Post's **Publish**
tab or **Generate** in **Reports**. If the action needs signatures, it doesn't run yet: a
signing request is created and its panel opens. If the same action is already waiting for
the same Post and the same content, you get that request instead of a new one.

The person who started a request can sign it too, unless the action's rule says
otherwise.

## The request panel

The panel opens on the right. It shows:

- the action, the Post and the country, for example "Election returns · Post A · Country
  B";
- the status and the expiry time, for example "Waiting · 1 of 3 · Expires at 20:08", and a
  progress bar;
- the rule in words, for example "Needs 3 signatures from Post A's signers, each with
  their digital certificate.";
- what is signed: a document card (name, SHA-256 hash and **Open the document**), or a
  details table for actions without a document;
- the **signing code**;
- the **Signers**: each person who can sign, with their title, and either "Signed *time*,
  Certificate *name*" or "Not signed". You are marked "(you)".

Its buttons are **Sign**, **Next member signs in** and, for the person who started it or
an operator, **Cancel request**.

To find a request later, use **Waiting for my signature** in the top header bar while
viewing an election event: anyone who can sign an action sees it, with the number of
requests left to sign. It appears as a signing icon with a badge when signatures are
pending; its tooltip names the action and the count.
It lists the event's requests waiting for the actions you can sign, in your Posts, with
their code, how many have signed, when they expire and whether you already signed. Choose
one to open its panel. A trustee finds their key share in the ceremony instead. With the
permission, **Signatures** > **Requests** lists every request.

The header count refreshes when entering or switching election events, opening the
list, or changing a request opened from that list. It does not automatically track
changes made by other users. Open the icon or refresh the page to update it.

## The signing dialog

**Sign** opens the dialog "Sign the *object*" with three steps.

### 1. Check

The dialog says who is signing ("You are signing as *name* · *title*, *Post*") and what is
signed: the document with its hash, or the details table. It shows the signing code;
everyone who signs sees the same code. For a document, check it and tick "I have checked
the *document*" before continuing.

The portal downloads the document and checks that its hash is the one the request signs.
It refuses to continue if they differ.

### 2. Certificate

1. Insert your security token and choose your certificate file. Use **Choose another
   file** to change it.
2. Type the certificate password (the eye shows it) and choose **Open certificate**. A file
   with strong encryption can take a few seconds to open.
3. The certificate card shows the holder's name, the issuer, the validity, the key type
   (RSA or EC P-256) and the SHA-256 fingerprint, followed by the checks the server will
   run (see [what every signature is checked for](./03-election_management_election-event_signatures_certificates.md#what-every-signature-is-checked-for)).
   On your first signature the list says "First use: it will be registered to you".
4. **Sign** is enabled only when every check passes.

### 3. Signed

The dialog shows the time, "with the certificate of *name*", the count ("1 of 3
signatures.") and who signs next ("Next: *names* sign."). When yours was the last
signature it says "All *n* signatures are in." and the action runs.

## The signing code

The signing code is derived from the request and from exactly what it signs. Everyone who
signs the same request sees the same code. When several people sign together, read it
aloud and compare: a different code means a different request or different content. The
code is also printed in each signature on signed documents and recorded in the logs.

## Next member signs in

On a computer shared by several signers, after you sign choose **Next member signs in**.
You are signed out; the next member signs in on the same computer and browser window and
returns to the same request, where the dialog opens for them. The request stays open until
its expiry time. The handover is logged as `SigningHandover`.

## Problems and what they mean

Most problems appear on the **Certificate** step, before you sign.

| Message | Meaning | What to do |
|---|---|---|
| Wrong password. Check it and try again. | The file refused the password. The attempt is logged without the file or the password. | Type it again. |
| This file is not a certificate file (.p12 or .pfx), or it is damaged. | The file can't be read. | Choose the certificate file from your security token. |
| This file has no private key. | The file has only a certificate. | Choose the `.p12` or `.pfx` file, not an exported certificate. |
| This certificate's key type is not supported. | Only RSA and EC P-256 keys can sign. | Ask for a supported certificate. |
| Not issued by a trusted issuer, with "Use the certificate *organization* registered for you. Certificates from other issuers are not accepted." | The certificate's issuer isn't trusted for this event. | Use the certificate on your own token, or ask the officer who manages certificates to import the issuer. |
| Not valid today | The certificate expired or isn't valid yet. | Ask for a new certificate. |
| Not made for signing | The certificate's key usage doesn't allow signing. | Ask for a signing certificate. |
| Revoked, or no current revocation list to check it | The certificate was revoked, or its issuer's list couldn't be downloaded and the event doesn't accept unchecked signatures. | Ask the officer who manages certificates. |
| Registered to *name*, with "This certificate can't sign for you. Use the certificate on your own security token." | The certificate, its key or its holder is registered to someone else. | Use your own certificate. |
| Not registered to you | The event registers certificates only through an officer, and yours isn't registered. | Ask the officer to register it. |
| Registered for another Post | With one Post per certificate, it already signed for another Post. | Use the certificate for this Post. |
| Already used for this request / You have already signed this request. | You, or this certificate, already signed it. | Nothing: the next signer signs. |
| The document changed while you were signing. Sign again. | Someone else signed the document at the same moment. | Sign again; the dialog prepares the current version. |
| This request changed after you opened it. | The request changed meanwhile, for example someone cancelled it. | Close the dialog and open the request again. |
| This request was cancelled: *reason*. Signatures given for it no longer count. | The request was cancelled (see [the reasons](./04-election_management_election-event_signatures_requests.md#statuses)): for example a recount changed the election returns, the rule changed, or a counted certificate was revoked. | Start the action again and sign the current version. |
| This request expired. | Not enough people signed in time. | Start the action again. |
| All signatures are in, but the action failed. | The signatures are recorded but the action couldn't run. | Check the [Logs](../11-election_management_election-event_logs.md) and start again. |

Every refused signature is logged as `SigningSignatureRefused` with the failed check.

## Notes per action

**Initialize voting.** Started with **Generate Initialization Report** in a Post's **Publish**
tab. When
enough members have signed, the Post is initialized and its Initialization Report is
generated. Whether that report also needs its own signatures is decided by **Generate
other election reports**.

**Open voting and Close voting.** Started with **Start voting** and **Stop voting** in a
Post's **Publish** tab. When enough members have signed, voting opens or closes at the
Post. After closing, the panel shows the closing time, the closing signatures and the
code; this closing record goes into the log. Pausing and whole-event manual controls
do not open this signing dialog. Scheduled opening or closing is refused and logged
when the corresponding action requires signatures; people must complete the protected
transition instead.

**Generate election returns and other reports.** In **Reports**, the **Signatures** column
says how many signatures each report needs ("Needs 3") or **Off**. The tally produces
election returns per Post and country and Initialization Reports per Post; when signatures
are required, these are signed through their requests rather than regenerated in Reports.
Participation reports are generated from Reports. Their held PDFs have a final page with
one field per required signer. Signers can inspect the document, but its released copy
cannot be downloaded, printed or transmitted until the request has executed successfully.
Each signature fills its field with the name, time and code, and PDF readers can validate it.

A report's configured password protection is applied after signing, as an encrypted file
containing the signed PDF. Download uses the password flow; direct **Print** is unavailable
for a protected report. The last signature alone does not mean release has finished:
wait for **Done**. The panel then offers **Download signed PDF**, and **Print** for an
unprotected report. **Transmit results** is offered for election returns when permitted.
A recount cancels an older waiting request whose report changed.

The download button is not restricted to the last signer. Anyone with access to the
completed request and the document-download permission can reopen it in **Signatures** >
**Requests** and download. The header and Reports request links show pending work, so they
do not provide a route back to an executed report. Staff without access to Requests need
that access to use this route. Reports does not list a history of generated files;
**Generate** creates another output and may require another set of signatures.

Generate other election reports currently protects participation reports and the tally's
Initialization Reports. Per-voter manual verification and activity logs have no signing
integration and continue through their existing unsigned generation paths.

A participation report can cover the whole election event when **Generate other election
reports** is Off. When signatures are required, select a Post in the report's **Election**
field. An existing event-wide report must be edited to select a Post before generating it;
the Reports screen explains this without starting a task. Preview remains available.

**Transmit results.** In **Tally** > **Transmission**, creating the package starts a signing
request for the Post and country instead of asking for a certificate file upload. Each
signer signs the package's results. When everyone has signed, the package carries their
signatures, and **Send to *n* destinations** becomes available: sending stays a separate
step. With the rule off, transmission works as before.

**Approve a voter manually.** In **Approvals**, approving an application starts a request
whose details table shows the application, why it needs a person, and the registry
record. When enough people have signed, the voter is approved and receives their
credentials. Rejecting an application doesn't ask for signatures.

**Approve a configuration version.** In **Publish**, publishing starts a request whose
details list the changes in the version: its digest, the signing rules changed since the
last publication, new scheduled events and whether ballots and contests changed.
The rule summary covers Off/signature-count changes, not a complete copy of signer roles,
requester eligibility, expiry or certificate checks. When the
configured officials have signed, the version is published. Generating a new version
cancels the waiting request.

**Confirm and contribute a key share (trustees).** In **Keys** (key ceremony) and **Tally**,
after you choose your key share file, the portal checks it is yours and starts a request
that signs only the file's hash; the key share itself is never part of the request. The
dialog opens for you at once. When you have signed, your step runs as usual, and your
signature is recorded with the ceremony. If the key share file is wrong, no request is
started.

## In the logs

Each step you take writes a USER entry with your name and a SYSTEM entry with what the
system checked, in the election event's
[Logs](../11-election_management_election-event_logs.md): starting, a file that didn't
open, signing (with your certificate and signature), a refusal, the handover, a
cancellation, completion and the action's result. See
[What is logged](./01-election_management_election-event_signatures.md#what-is-logged).
