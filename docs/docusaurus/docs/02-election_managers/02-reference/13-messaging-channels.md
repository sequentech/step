---
id: messaging_channels
title: Messaging Channels
sidebar_position: 13
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Voters can receive their verification codes, enrollment results, credential
notices and reminders by **WhatsApp**, **Viber** or **Facebook Messenger**, as
well as by email and SMS. Every channel sends from an account an
administrator configures, and each election event chooses which account
each channel uses and what each election offers.

## Channels

| Channel | Provider | Recipient | Codes | Notices |
|---|---|---|---|---|
| Email | Amazon SES or SMTP | Email address | Code in an email | Free text |
| SMS | Amazon SNS | Phone number (E.164) | Transactional SMS | Free text |
| WhatsApp | WhatsApp Cloud API | Phone number (E.164) | Approved authentication template | Approved utility templates |
| Viber | Viber Business Messages through Infobip | Phone number (E.164) | Partner-approved OTP template | Partner-approved templates |
| Messenger | Messenger Send API | Page-scoped ID | A message in a conversation the voter started | A message within 24 hours of the voter's last interaction, or an approved utility template after it |

Any channel can also send through a **configurable HTTP provider**: an
account that describes the provider's API instead of using a built-in one.
It serves another Viber partner, a WhatsApp Solution Provider's own API, or
an SMS or email gateway, with no change to the platform. See
[Configurable provider](#configurable-provider).

Facebook and Messenger are one channel: a Facebook Page messages the voter,
who reads it in Facebook or Messenger. Public Page posts are not messages to
a voter and are not used.

Without any sending account, an event keeps sending email and SMS through
the platform's default transports, as before.

## Sending accounts

**Settings > Messaging** lists the tenant's sending accounts. Viewing them
needs `messaging-account-read`; adding, editing, checking and testing them
needs `messaging-account-write`.

Each account shows three separate states:

- **Connected**: the credentials work.
- **Ready for OTP** and **Ready for notices**: the purpose can be enabled
  for an event. A connected account is not ready while any prerequisite is
  missing, and the screen names it:
  - *Needs provider approval*: WhatsApp accounts start as pending. Meta
    only allows government messaging through an approved arrangement, so
    the account's approval must be confirmed before any purpose is enabled.
  - *Needs production access*: the provider account is still in a sandbox
    (SES, SNS), the WhatsApp number is not verified, or the app is not
    subscribed to the Facebook Page.
  - *Needs approved template*: WhatsApp and Viber only send approved
    templates. WhatsApp approvals are read from Meta on each check. Viber
    approvals are entered in the account, per purpose and language, once
    the partner approves them, because Infobip's template API is not
    generally available.

**Readiness** decides where those answers come from:

- *From the provider's check* (default): the connection check decides.
- *Confirmed by an administrator*: the administrator confirmed with the
  provider that the account is connected, in production and has its
  templates approved. Use it when the provider's check cannot tell, or the
  provider's arrangement differs from what the check expects. Provider
  approval is still confirmed separately.

Credentials are write-only. After saving, the account only shows when each
was last replaced. For WhatsApp and Messenger the account shows the callback
path to enter at Meta and generates the verify token Meta asks for; the
token is shown once.

**Check connection** asks the provider about the account and stores the
result. **Send test message** sends one message for a chosen purpose and
destination. *Accepted* means the provider took the message, not that it
was delivered.

Accounts also take a sending rate, a share of it kept for codes so that a
bulk reminder never delays a voter waiting for a code, and the country
calling codes the account may send to.

WhatsApp and Messenger accounts take an optional **API base URL**, for a
provider that serves the same API from its own address.

### Configurable provider

A configurable account is entered as JSON and holds:

| Part | What it describes |
|---|---|
| `send` | The request that sends one message: method, URL, headers and JSON body |
| `message_id_pointer` | Where the provider's message ID is in the answer |
| `phone_format` | `E164` (`+639171234567`) or `DIGITS` |
| `template_required_for` | Purposes that only send approved templates |
| `conversation_window_hours` | Hours after the voter's last message in which free text may be sent |
| `approved_templates` | Languages with an approved template, per purpose |
| `check` | A request that succeeds when the credentials work |
| `token` | A request that obtains a short-lived token, and where the token is in the answer |
| `jwt` | Claims and algorithm (`RS256` or `HS256`) of a token signed with the `API_SECRET` credential |
| `reports` | How delivery reports are authenticated and read |
| `reconcile` | A request that asks the provider about one message |

Only `send` is required. URLs, headers and bodies take placeholders:
`{{to}}`, `{{text}}`, `{{subject}}`, `{{html}}`, `{{code}}`, `{{template}}`,
`{{language}}`, `{{message_id}}`, `{{callback_url}}`, `{{param.1}}`…,
`{{parameters}}` (a list), `{{named_parameters}}` (an object),
`{{credential.NAME}}`, `{{basic_auth}}`, `{{token}}` and `{{jwt}}`.
Credentials are `API_KEY`, `API_SECRET`, `USERNAME`, `PASSWORD`,
`ACCESS_TOKEN` and `WEBHOOK_SECRET`, stored write-only like any other.

```json
{
  "provider": "HTTP_API",
  "label": "COMELEC",
  "send": {
    "url": "https://api.example.com/viber/messages",
    "headers": {"Authorization": "Bearer {{credential.API_KEY}}"},
    "body": {
      "to": "{{to}}",
      "template": "{{template}}",
      "language": "{{language}}",
      "parameters": "{{parameters}}",
      "reference": "{{message_id}}",
      "callback": "{{callback_url}}"
    }
  },
  "message_id_pointer": "/id",
  "template_required_for": ["OTP", "NOTICE"],
  "reports": {
    "auth": {"kind": "HMAC_SHA256", "header": "X-Signature"},
    "items_pointer": "/reports",
    "status": {
      "message_id_pointer": "/id",
      "state_pointer": "/status",
      "states": {"delivered": "DELIVERED", "failed": "FAILED"}
    }
  }
}
```

Reports are authenticated by the unguessable callback address alone
(`URL_KEY`), a header equal to `WEBHOOK_SECRET` (`HEADER_SECRET`), an
HMAC-SHA256 of the body or of a composed text (`HMAC_SHA256`), or a signed
token (`JWT_HS256`). Providers that report with a `GET` request and query
parameters are read the same way. A provider that returns no message ID is
matched by the `{{message_id}}` reference sent with the message.

## Election event settings

The **Messaging** tab of an election event needs `messaging-config-write`
to edit. For each channel it selects an account and has separate **OTPs**
and **Notices** switches. Saving checks every reference and prerequisite and
names what is missing; nothing is saved while anything is wrong.

- **Template bindings** map each purpose and language to the provider's
  approved template. A binding may name one message (a message key, such
  as `otp` or a template's alias) and the provider's own language code
  when it differs, such as `en_US` for `en`. The most specific binding
  wins: the message in the voter's language, then the message in any
  language, then the purpose in the voter's language, then the purpose.
  A message template can also carry its own approved template name.
  Template parameters are sent in order, or by name when written
  `@name=value`.
- **Outside the conversation window** chooses, for Messenger and any
  provider with a conversation window, between waiting for the voter to
  write again and sending an approved utility template.
- **Fallback for notices** is the order in which a notice moves to another
  channel after a confirmed failure, among the channels the voter verified.
  Codes never fall back on their own: the voter chooses another way.
- **Channels by election** restricts the channels an election offers to a
  subset of the event's channels.
- **Reply to incoming messages** is the text sent, at most once a day per
  sender, when a voter writes to an account.
- **Delivery** shows, per channel, how many messages are queued, accepted,
  delivered, failed and unknown. A provider without delivery reports (SNS
  for SMS) shows delivery as unavailable, never as zero delivered.

Saving publishes the channels voters may see to the event's Keycloak realm
(`sequent.messaging`); account identifiers and credentials never reach the
voter pages.

## Voters

At enrollment a voter chooses how to get codes among the channels their
Post offers, enters the number for WhatsApp, Viber or SMS, and agrees to the
consent text that names the sending organization and channel. A code then
proves the contact. Choosing a channel for codes does not change where
notices go; that is a separate, explicit choice.

For Messenger, the voter opens the Page's conversation from the link or QR
code shown, and taps Get Started (or types the linking word). The code
arrives in that conversation and must be entered in the same browser
session; only then is the Messenger contact saved.

After enrollment, a signed-in voter adds or replaces a WhatsApp, Viber or
Messenger contact through the `messaging-app-otp-ra` required action, which
verifies the new contact with a code before saving it. The action must be
registered and enabled in the event's realm.

At sign-in only contacts the voter already verified are offered. **Get the
code another way** replaces the code; the resend wait and the attempt limit
still apply.

## Delivery states

| State | Meaning |
|---|---|
| Queued | Recorded, not yet handed to the provider |
| Accepted | The provider took the message |
| Delivered | The provider reported delivery |
| Failed | The provider refused it or reported a failure |
| Unknown | The request may have reached the provider but its answer was lost |

An unknown outcome is never treated as a failure: the platform asks the
provider when it can, and otherwise leaves it for review. It never resends a
message blindly, so a voter is not messaged twice for one notice. Exactly-once
delivery cannot be guaranteed by any provider.

## Records

Every attempt is recorded with the channel, account, purpose, masked
recipient, provider message ID, state and the billing data the provider
reports, so monthly volumes per channel and country can be reconciled with
invoices. Codes, message bodies and credentials are never recorded.

## Going live

Every provider agreement is applied by configuration; none needs a new
version of the platform.

- **WhatsApp**: once Meta and the Solution Provider confirm the operating
  arrangement, enter the credentials and mark the account's provider
  approval as confirmed. A Solution Provider with its own API is entered as
  a configurable provider, or with its API base URL.
- **Viber**: enter the partner's credentials. Infobip is built in; any
  other partner is a configurable provider.
- **Messenger outside the 24-hour window**: once Meta approves utility
  templates for the Page, bind them in the event and choose to send utility
  templates outside the window. Until then, notices outside the window use
  another eligible channel, and codes need a fresh interaction.
- Which channels each Post offers, the consent wording and how long message
  records are kept.
