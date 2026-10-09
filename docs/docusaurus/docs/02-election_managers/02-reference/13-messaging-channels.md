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
well as by SMS and email. Every channel sends from an account an
administrator configures, and each election event chooses the account each
channel uses and the channels each election offers.

Each voter has **one channel**. The voter chooses it and proves it with a
code at enrollment, and every code and notice for that voter goes there and
nowhere else. The accounts only send: they never answer messages voters
write to them.

## Channels

Channels are always listed in this order: WhatsApp, Viber, Messenger, SMS,
email.

| Channel | Provider | Recipient | Codes | Notices |
|---|---|---|---|---|
| WhatsApp | WhatsApp Cloud API | Phone number (E.164) | Approved authentication template | Approved utility templates |
| Viber | Viber Business Messages through Infobip | Phone number (E.164) | Partner-approved OTP template | Partner-approved templates |
| Messenger | Messenger Send API | Page-scoped ID | A message in the chat the voter opened with Get Started | A message within 24 hours of the voter's Get Started, or an approved utility template after it |
| SMS | Amazon SNS | Phone number (E.164) | Transactional SMS | Free text |
| Email | Amazon SES or SMTP | Email address | Code in an email | Free text |

Facebook and Messenger are one channel: a Facebook Page messages the voter,
who reads it in Facebook or Messenger. Public Page posts are not messages to
a voter and are not used.

Each provider has its own built-in adapter. A provider that is not listed
here needs a new adapter in the platform; it can't be described by
configuration.

Without any sending account, an event keeps sending email and SMS through
the platform's default transports, as before.

## Sending accounts

**Settings > Messaging** lists the tenant's sending accounts. Viewing them
needs `messaging-account-read`; adding, editing, checking, testing and
deleting them needs `messaging-account-write`.

Each account shows three separate states, with the time of the last check
and a short reason when one is missing:

- **Connected**: the provider accepted the credentials.
- **Ready for OTP** and **Ready for notices**: the purpose can be enabled
  for an event. A connected account is not ready while any prerequisite is
  missing, and the screen names it:
  - *Needs provider approval*: WhatsApp accounts start as pending until
    the organization's arrangement with Meta is approved (see
    [Going live](#going-live)).
  - *Needs production access*: the provider account is still in a sandbox
    (SES, SNS), the WhatsApp number is not verified, or the app is not
    subscribed to the Facebook Page.
  - *Needs approved template*: WhatsApp and Viber only send approved
    templates. WhatsApp approvals are read from Meta on each check. Viber
    templates are entered on the account, per purpose and language, with
    the ID the partner gave when it approved them, because Infobip's
    template API is not generally available.

Readiness always comes from the provider's answers to the connection check.
An administrator can't declare an account ready. An account saved with the
earlier "Confirmed by an administrator" setting is now read from its check
like any other.

Credentials are write-only. After saving, the account only shows when each
was last replaced. A new credential is checked with the provider before it
is saved; until the check connects, the previous credential stays in use.
Changing the account's API address requires entering the credentials again,
and the address must use `https`.

**Check connection** asks the provider about the account and stores the
result; accounts are also checked once a day. **Send test message** sends
one message for a chosen purpose and destination. *Accepted* means the
provider took the message, not that it was delivered.

Accounts also take a sending rate, a share of it kept for codes so that a
bulk reminder never delays a voter waiting for a code, and the country
calling codes the account may send to. Without a sending rate the account
is not throttled, and the account says so.

An account can't be deleted while an election event uses it.

### Callback URL

Delivery reports, and Messenger's Get Started, reach the platform through
one public address. Each account that receives callbacks shows its own
**callback URL** in its drawer, to enter at the provider:

- **WhatsApp and Messenger**: the callback URL and a **verify token** go in
  the Meta app's webhook settings. The verify token is generated on the
  account and shown once. Reports are checked with the app secret.
- **Viber**: see [Viber delivery reports](#viber-delivery-reports).
- **Email through SES**: see [Email receipts](#email-receipts).

If the deployment has no public callback address configured, the account
shows *Callback address not configured*. Delivery for its channel then
shows as unavailable, never as zero delivered, and Messenger is not ready
for codes, because Messenger codes need Get Started.

### Messenger Get Started

A Page can only message a voter who opened the chat. Each time a voter
needs a code by Messenger, the voter page shows a link to the Page (and a
QR code on computers). The voter taps **Get Started** in the chat, and the
Page answers with the code and nothing else. Get Started also opens
Messenger's 24-hour window for notices.

For this to work:

- Enter the Page ID and the Page **username**: the voter link is
  `m.me/<username>`.
- In the Meta app, subscribe the app to the Page and enter the callback URL
  and verify token.
- Set the Page's Get Started button.
- Grant the `pages_messaging` permission.

The connection check reports each of these.

Changing an account's Page disconnects the voters connected to the old
Page, because Page-scoped IDs belong to a Page. The drawer shows how many
voters are affected before saving; they get codes again only after
connecting Messenger again.

### Viber delivery reports

Infobip sends Viber delivery reports through a subscription (notification
profile) that you create in Infobip:

1. Save the Viber account. It generates a **report username and
   password**, shown once.
2. In Infobip, create a subscription for Viber delivery reports with the
   account's callback URL, and choose **Basic** authentication with that
   username and password.

A report without those credentials, or with wrong ones, is refused. The
platform doesn't send a report address with each message, so reports only
come through the subscription.

### Email receipts

SES delivery receipts arrive through an Amazon SNS topic:

1. In SES, send the sending identity's delivery, bounce and complaint
   notifications to an SNS topic.
2. In SNS, subscribe the account's callback URL to that topic over HTTPS.
   The platform confirms the subscription by itself.
3. Enter the topic's ARN as the account's **notification topic**.

Without a notification topic, email delivery shows as unavailable. SMTP and
SNS (SMS) accounts have no receipts: their delivery always shows as
unavailable, never as zero delivered or failed.

### Unsupported providers

The configurable HTTP provider and the "nothing is sent" console provider
have been removed. An account saved with either one still appears in the
list with *Unsupported provider: replace this account*, and nothing is sent
from it. Create an account with a supported provider, choose it in each
event that used the old one, then delete the old account.

## Election event settings

The **Messaging** tab of an election event shows the channels the event
offers and how their messages are doing. Viewing it needs
`messaging-config-read`; editing it needs `messaging-config-write`.

For each channel the tab selects an account and has separate **OTPs** and
**Notices** switches. A switch that can't be turned on is disabled with the
missing prerequisite, such as "Add an account in Settings > Messaging
first". Saving checks every reference and prerequisite and names what is
missing; nothing is saved while anything is wrong.

| Column | What it shows |
|---|---|
| Channel | The channel, with its icon |
| OTPs, Notices | Whether codes and notices are sent on this channel |
| Sends from | The account |
| Voters using it | Voters of the event whose channel this is |
| Sent, last 24 h | Messages sent in the last 24 hours |
| Delivered | Messages the provider reported delivered, or *Unavailable* when the account has no receipts |
| Failed | Messages that failed after their retries. Opens the event's Logs, filtered to this channel's failed messages |
| Unknown | Messages whose outcome is not confirmed yet |
| Unresolved | Unknown messages older than 24 hours (see [Unknown and Unresolved](#unknown-and-unresolved)) |

Counts are per message, not per attempt: a message retried twice counts
once.

- **Template bindings** map each purpose and language to the provider's
  approved template. A binding may name one message (a message key, such
  as `otp` or a template's alias) and the provider's own language code
  when it differs, such as `en_US` for `en`. The most specific binding
  wins: the message in the voter's language, then the message in any
  language, then the purpose in the voter's language, then the purpose.
  A message template can also carry its own approved template name.
  Template parameters are sent in order, or by name when written
  `@name=value`.
- **Messenger outside the 24-hour window**: notices use the approved
  utility template bound on the event. Until one is bound, Messenger
  notices can't be enabled, and the switch says why. A code outside the
  window always needs a fresh Get Started.
- **Channels by Post** restricts the channels each election (Post) offers
  to a subset of the event's channels. New Posts start with the event's
  enabled, ready channels. Untick a channel for a Post where it does not
  work.

Saving publishes the channels voters may see to the event's Keycloak realm
(`sequent.messaging`); account identifiers and credentials never reach the
voter pages.

## Voters

### What voters see

- **Enrollment**: under *Get codes and updates by*, the voter picks one of
  the channels their Post offers, enters the number for WhatsApp, Viber or
  SMS, and agrees to the consent text that names the sending organization
  and channel. When the voter changes Post, the list changes with it.
- **Verify contact**: the code page names the channel and the masked
  destination, such as *We sent a code to your WhatsApp, +966 ••• ••• 567*,
  and offers **Resend code** on the same channel. There is no other way to
  get the code. Verifying the code saves the channel; the same channel
  carries codes and notices.
- **Messenger**: the voter opens the Page from the link or QR code and taps
  Get Started. The code arrives in that chat and must be entered in the
  same browser session; only then is the Messenger contact saved. Opening
  the link alone neither verifies the contact nor lets the Page send.
- **Pending review**: the page names the channel the result will be sent
  to.
- **Sign-in**: the code goes only to the voter's channel, and the page
  names it.
- **Password reset**: the reset code or link goes to the voter's channel.
  The page always says *If an account exists, we sent the link to its
  contact*, without naming a channel.

Masked destinations show the country code and the last three digits of a
number, or the first two characters of an email address.

The voter may wait 60 seconds between resends, and a failed send starts
that wait too. After 5 wrong codes the attempt ends with *Too many
attempts. Start again.*, and the voter starts the sign-in or enrollment
again. Resending a code never resets the count.

### How a voter's channel is read

The platform reads one channel per voter, the same way everywhere
(enrollment, sign-in, notices, the voter drawer):

1. A voter who verified messaging contacts in the earlier release: the
   channel they had chosen for notices if it was verified, else their first
   verified channel in channel order.
2. A voter enrolled with one channel: that channel, if its number, address
   or Messenger contact is present.
3. Any other voter: email if the voter has an email address, else SMS if
   they have a mobile number.

Voters in the first case are rewritten to a single channel by a realm
migration; the migration logs every voter it changes and why.

### Changing the channel

A signed-in voter changes the channel from **Change how we contact you** in
the voting portal's user menu. The voter must have signed in within the
last 5 minutes, or is asked to sign in again. The page lists the Post's
channels, sends a code to the new channel, and when the code is verified
**replaces** the old channel completely. The required action
`message-channel-ra` must be registered and enabled in the event's realm;
realms that registered it as `messaging-app-otp-ra` keep working.

### Voter drawer and Logs

The voter drawer's **Messaging** section shows the voter's channel, the
masked number or Messenger Page, when consent was given, and the **last
failure**: its state, time and reason.

The voter's **Logs** dialog lists every message with its channel and its
**Status** from the message records.

### Send Notification

Send Notification has no channel choice: every notice goes to each voter's
own channel. The **Each voter's channel** table shows, per channel, how many
of the selected voters use it and the account it sends from:

> Each voter gets one message on their own channel. If delivery fails, it is
> retried on that channel and the failure shows on the event's Messaging
> tab. Unconfirmed delivery is shown as Unknown.

Sending to selected voters with no voter chosen is refused.

## Delivery

| State | Meaning |
|---|---|
| Queued | Recorded, not yet handed to the provider |
| Accepted | The provider took the message |
| Delivered | The provider reported delivery |
| Failed | The provider refused it or reported a failure, or it could not be sent |
| Unknown | The request may have reached the provider, but its answer was lost |

A message is never sent on another channel. When it can't be sent on the
voter's channel (the Post doesn't offer the channel, there is no content for
it, there is no account, or the voter has no contact), it is recorded as
failed with that reason, and nothing else is sent.

A temporary failure, such as the provider being busy, is retried on the
same channel after a growing wait, up to three attempts in all. After that
the message stays failed and shows on the event tab, in Logs and in the
voter drawer. Codes are never retried by the platform: the voter presses
**Resend code**.

A missing receipt is never treated as a failure, and a late report never
moves a delivered message back.

### Unknown and Unresolved

*Unknown* means the platform can't tell whether the provider took the
message: for example, the provider didn't answer in time. Such a message is
**never resent automatically**, because the voter may already have it.

- Where the provider can be asked (Viber through Infobip), the platform asks
  it again regularly and updates the message.
- An Unknown message older than 24 hours is shown as **Unresolved**:
  delivery couldn't be confirmed. It is not counted as failed, because it
  may have arrived.

What to do with Unknown and Unresolved messages:

1. On the event's Messaging tab, check whether one channel or account has
   most of them. On the account in Settings > Messaging, run **Check
   connection**; a provider outage or expired credential shows there.
2. In the voter's Logs dialog, find the message and its provider message
   ID, then look it up in the provider's own console or delivery logs.
3. For a code, nothing is needed: a voter who didn't get it presses
   **Resend code**.
4. For a notice, ask the voter (through the helpdesk) whether it arrived.
   If it didn't, and it still matters, send the notice again to that voter
   from Send Notification. That is a new message, recorded on its own.
5. Never record an Unresolved message as failed: it may have been
   delivered.

### Reasons you may see

| Reason | Where | What it means |
|---|---|---|
| Callback address not configured | Account | The deployment has no public callback address; receipts are unavailable and Messenger is not ready for codes |
| Unsupported provider: replace this account | Account | The account uses a removed provider and sends nothing |
| Enter the credentials again for the new address | Account | The API address changed without new credentials |
| No contact channel | Message | The voter has no channel to send to |
| Can't be retried: no stored template | Message | A failed message whose content can't be rebuilt; it stays failed |

## Records

Every attempt is recorded with the channel, account, purpose, masked
recipient, recipient country, provider message ID, state and the billing
data the provider reports, so monthly volumes per channel and country can
be reconciled with invoices. The electoral log records each attempt's
channel, account, purpose and outcome with the masked recipient. Codes,
message bodies, credentials and messages voters send to the accounts are
never recorded.

## Going live

Every provider agreement is applied by configuration, and none needs a new
version of the platform for a supported provider. The deployment
prerequisites are listed in
[Messaging deployment](../../07-developers/14-messaging/02-messaging-deployment.md#deployment-prerequisites).
In short:

- **WhatsApp**: the organization holds the Meta business portfolio, the
  WhatsApp Business Account and the number, through a Solution Partner.
  Once Meta approves the arrangement and the templates, enter the
  credentials and record the account's provider approval. A Solution
  Partner that serves the Cloud API from its own address is entered with
  its API base URL.
- **Viber**: choose the partner. Infobip is built in; another partner needs
  an adapter. Enter the partner's credentials and approved templates, and
  create the report subscription.
- **Messenger outside the 24-hour window**: once Meta approves utility
  templates for the Page, bind them on the event.
- Which channels each Post offers, the consent wording and how long message
  records are kept.
