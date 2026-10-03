---
id: election_management_election_event_signatures
title: Signatures
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Some actions of an election event can be **protected**: they run only after enough
authorized people have signed them with their digital certificates. The election event's
**Signatures** tab, after **Keys**, decides which actions need signatures, how many, and
who may sign.

When a protected action needs signatures, starting it creates a **signing request**
instead of running it. Each signer opens the request, checks what is signed and signs it
with the certificate file on their security token. The action runs once, when the last
required signature arrives. Every step is recorded in the election event's
[Logs](../11-election_management_election-event_logs.md).

Signing happens in the signer's browser: the certificate file, its private key and its
password never leave the signer's computer. Only the signature and the public certificate
are sent. The signer is also signed in with their own account, so each signature is tied
to both the person's account and their certificate.

## Terms

| Term | Meaning |
|---|---|
| Post | An election of the election event. The screens say "Post" by default; an organization can rename it, and the actions, with its translation overrides (**Settings** > **Localization**). |
| Protected action | One of the actions in the table below. The list is part of the product; whether each needs signatures is the election event's configuration. |
| Rule | An action's settings in this election event: whether it needs signatures, how many, whether the person who starts it can also sign, and when a request expires. |
| Signing request | One run of a protected action waiting for its signatures. It names the Post (and country, where it applies), exactly what is signed, who started it and when it expires. |
| Signing code | A short code such as `7F3A-91C2` that every signer of a request sees, so the people signing together can compare it aloud. |
| Trusted issuer | A certificate authority whose certificates may sign. Staff issuers are separate from the certificate authorities voters sign in with ([Certificates](../15-election_management_election-event_certificates.md)). |

## Protected actions

| Action | Started in | What is signed | When enough people have signed |
|---|---|---|---|
| Initialize voting | Post > **Publish**, **Generate Initialization Report** | The Post and its published configuration | Initializes the Post and generates its Initialization Report. |
| Open voting | Post > **Publish**, **Start voting** | The Post and the channel | Opens voting at the Post. |
| Close voting | Post > **Publish**, **Stop voting** | The Post and its channels | Closes voting. The closing record lists the closing signatures. |
| Generate election returns | **Reports** | The election returns PDF of a Post and country | Releases the signed PDF for download, printing and transmission. |
| Generate other election reports | **Reports** | The Initialization Report or participation report PDF; per-voter manual verification and activity logs are not integrated | Releases the signed report. |
| Transmit results | **Tally** > **Transmission** | The results package of a Post and country, and its destinations | Builds the signed package; it can then be sent. |
| Approve a voter manually | **Approvals** | The application, the registry record and the decision | Approves the voter and issues their credentials. |
| Approve a configuration version | **Publish** | The changes in the version and their digest | Publishes the version. |
| Confirm a key share (key ceremony) | **Keys**, by each trustee | The ceremony, the trustee and the key share's hash | Lets the trustee's key step run and records the signature with the ceremony. |
| Contribute a key share (tally) | **Tally**, by each trustee | The tally, the trustee and the key share's hash | Lets the trustee's contribution run and records the signature. |

Every action starts **Off**. An action that is off runs exactly as it does without this
feature. [Signing a protected action](./05-election_management_election-event_signatures_signing.md)
describes each action from the signer's side.

## The tab

The tab has three sub-tabs. Each has its own permissions, so each can belong to a
different role:

| Sub-tab | What it holds | Read permission |
|---|---|---|
| [Protected actions](./02-election_management_election-event_signatures_protected-actions.md) | The rule of each action and who can sign it. | `signing-rules-read` |
| [Certificates](./03-election_management_election-event_signatures_certificates.md) | Trusted issuers, the certificate checks, and the certificates registered to people. | `signing-certificates-read` |
| [Requests](./04-election_management_election-event_signatures_requests.md) | Every signing request of the election event. | `signing-requests-read` |

The tab itself needs `election-event-signatures-tab` and at least one of the read
permissions. A sub-tab is absent without its read permission. Without the matching write
permission its controls are hidden or disabled and the sub-tab is marked **Read only**.
Signers need none of these: the request panel carries what they need. See
[Signature permissions](./06-election_management_election-event_signatures_permissions.md).

## Configuration version and lockdown

The rules belong to the election event's configuration. The Protected actions sub-tab
shows the configuration version they are part of and who changed them last, and a rule
change becomes part of the next configuration version. While the election event is locked
down, rules can't be edited.

The rules and the certificate checks travel with the election event when it is exported
and imported. The export also carries the staff issuers in their own PEM file, apart from
the voters' certificate authorities. Importing an event whose Posts carry the older
per-Post transmission threshold (`miru:area-threshold`) and no Transmit results rule
creates that rule when every Post has the same threshold; when they differ, no rule is
created, each Post keeps its own threshold, and the import says which Posts differ.

## What is logged

Every step of a signing request, and every change in the tab, writes **two** entries to
the election event's [Logs](../11-election_management_election-event_logs.md), with the
same statement kind:

- a **USER** entry, attributed to the person who took the step (for an expiry, the person
  who started the request);
- a **SYSTEM** entry, with what the system checked or did. A refusal or a failure is
  logged with log type ERROR.

Request entries carry the Post, the country, the action, the request and its signing
code. Signature entries also carry the certificate (subject, issuer, serial, fingerprint)
and the signature itself, so the log alone shows who authorized each action. The Logs
tab's **Event type** column tells USER and SYSTEM entries apart; its filters and its export
include these entries.

| Step | Statement kind |
|---|---|
| A request is started | `SigningRequestCreated` |
| A certificate file doesn't open (never the file or its password) | `SigningCertificateOpenFailed` |
| Someone signs | `SigningRequestSigned` |
| A signature is refused | `SigningSignatureRefused` |
| A certificate is registered (on first use or by an officer) | `SigningCertificateRegistered` |
| Next member signs in | `SigningHandover` |
| A request is cancelled | `SigningRequestCancelled` |
| A request expires | `SigningRequestExpired` |
| The last signature arrives | `SigningRequestCompleted` |
| The action runs (or fails) | `SigningActionExecuted` |
| A rule changes | `SigningRuleChanged` |
| Who can sign changes | `SigningPermissionChanged` |
| A trusted issuer is imported or removed | `SigningIssuerChanged` |
| The certificate checks change | `SigningChecksChanged` |
| A certificate is revoked | `SigningCertificateRevoked` |
| The requests are exported | `SigningRequestsExported` |

Developers: see [Signing architecture](../../../../07-developers/14-signing/01-signing-architecture.md).
