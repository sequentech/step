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
was last replaced. The account and the credentials typed with it are saved
together, in one step: either both are stored or neither is, and the drawer
shows the reason when the save is refused.

Changing the address the credentials are sent to (the Graph API base URL,
the Infobip API base URL or the SMTP server) needs the credentials again,
because the stored ones would otherwise go to the new address. As soon as
the address changes, the drawer opens those credential fields and says
*Enter the credentials again for the new address*; it doesn't save until
they are typed. The address must start with `https://`.

**Check connection** asks the provider about the account and stores the
result. **Send test message** sends one message for a chosen purpose and
destination. *Accepted* means the provider took the message, not that it
was delivered.

Accounts also take a sending rate, a share of it kept for codes, and the
country calling codes the account may send to.

Deleting an account doesn't check whether an election event still uses it.
Before deleting one, choose another account in the events that use it:
otherwise their messages on that channel fail with *No sending account for
the channel* (email and SMS then go out through the deployment's own
sender).

:::note Not available yet

These account features are planned and not in this release:

- checking a new credential with the provider before saving it, keeping the
  previous one until the check connects;
- a daily connection check of every account (today the check runs when you
  press **Check connection**);
- one sending rate shared by every process that sends for the account, with
  its share for codes kept free during bulk sends, and the account saying
  when it has no rate;
- refusing to delete an account an election event uses.

:::

### Callback URL

Delivery reports, and Messenger's Get Started, reach the platform through
one public address. Each account that receives callbacks shows its own
**callback URL** in its drawer, to enter at the provider:

- **WhatsApp and Messenger**: the callback URL and a **verify token** go in
  the Meta app's webhook settings. The verify token is generated on the
  account and shown once. Reports are checked with the app secret.
- **Viber**: see [Viber delivery reports](#viber-delivery-reports).
- **Email through SES**: see [Email receipts](#email-receipts).

If the deployment has no public callback address configured, the account's
drawer shows *Callback address not configured* instead of a URL. Delivery
for the account's channel then shows as *Unavailable* (never as zero
delivered), with that reason, and a Messenger account is not *Ready for
OTP*: Messenger codes need Get Started, which arrives as a callback. A
Viber account whose report credentials were never generated also shows
delivery as *Unavailable*, with *Generate the report credentials and create
the Infobip subscription*. If the callback URL can't be loaded at all, the
drawer says so.

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
Page, because Page-scoped IDs belong to a Page. The drawer warns about it
before saving; those voters get codes again only after connecting Messenger
again. (Showing how many voters are affected is planned, not yet
available.)

### Viber delivery reports

Infobip sends Viber delivery reports through a subscription (notification
profile) that you create in Infobip:

1. Save the Viber account, then press **Generate report credentials** in
   its drawer. The **report username and password** are shown once, with
   the callback URL and these steps.
2. In Infobip, create a subscription for Viber delivery reports with the
   account's callback URL, and choose **Basic** authentication with that
   username and password.

Generating them again replaces them at once, so the drawer asks first when
they are already set: until the Infobip subscription has the new ones,
Viber reports are refused and messages stay *Accepted*.

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
`messaging-config-read`; editing it needs `messaging-config-write` as well.
A user with only `messaging-config-write` sees the tab with a note that
reading the settings needs `messaging-config-read`. On upgrade, existing
realms get the messaging permissions, and the groups `admin` and
`admin-light` get `messaging-config-read` and the account permissions, as
new realms do; any group that already holds `messaging-config-write` or
`messaging-account-read` also gets `messaging-config-read`.

An event whose settings were copied from the earlier release's storage and
don't pass today's checks shows *Configuration needs saving again*: fix the
problems the tab shows, then save. Until then, sends still check every
reference they use.

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
| Delivered | Messages the provider reported delivered, or *Unavailable* when the account has no receipts or no callback address |
| Failed | Messages whose latest attempt failed. Opens the event's Logs, filtered to this channel's failed messages |
| Unknown | Messages whose outcome is not confirmed yet |
| Unresolved | Unknown messages older than 24 hours (see [Unknown and Unresolved](#unknown-and-unresolved)) |

Counts are per message, by the state of its latest attempt: a message
retried twice counts once. Logs filters on the same thing: the **Status**
filter matches each message's current state, so the Failed link opens the
same messages the count shows. Failures of voters with no contact channel
are never counted under a channel; they show on a row of their own, *No
contact channel*, whose failed count opens Logs filtered to them (the
Channel filter's *No contact channel* choice).

:::note Not available yet

The message counts (Voters using it, Sent last 24 h and the states) come
from an action that is planned and not in this release. Until it is
available, the tab says the counts could not be loaded and shows dashes,
never zero.

:::

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
  to, and a reference code to give support, such as `Q5KW-8RFX` (shown
  once the backend returns the application's ID to Keycloak; not yet in
  this release). The reference comes from the application's ID only. Staff
  find the application by typing the reference in the **Approvals**
  search box, in any case, with or without the dash.
- **Sign-in**: the code goes only to the voter's channel, and the page
  names it.
- **Password reset**: the reset code or link goes to the voter's channel.
  The page always says *If an account exists, we sent the link to its
  contact*, without naming a channel. The reset code page is the same
  whether the account exists or not (the code step's setting
  `code-page-disclosure` is `NEUTRAL` on reset flows): it names no channel
  and shows no masked destination. Because that page shows no Messenger
  link either, a voter whose channel is Messenger can't get a reset code
  there and asks support.
- **Temporary credentials**: a new temporary password takes effect only
  when its notice was handed to the provider (queued, accepted, delivered
  or unknown). When the send is refused or fails, the password stays
  unchanged; the page stays neutral and the Keycloak event records the
  reason.

Masked destinations show the country code and the last three digits of a
number, or the first two characters of an email address.

The voter may wait 60 seconds between resends, and a failed send starts
that wait too. Wrong codes are counted per code step: after 5 the step ends
with *Too many attempts. Start again.*, and the voter starts the sign-in or
enrollment again. Resending a code never resets the count; only starting
again does. Changing the channel has its own wait and its own count.

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
the voting portal's user menu; the item appears only when the event offers
at least one channel for codes (the portal reads the realm's public
`GET /realms/<realm>/messaging-channels`, which lists only channel names). The voter must have signed in within the
last 5 minutes, or is asked to sign in again. The page lists the Post's
channels, sends a code to the new channel, and when the code is verified
**replaces** the old channel completely. The required action
`message-channel-ra` must be registered and enabled in the event's realm;
realms that registered it as `messaging-app-otp-ra` keep working.

After the change, the **old** channel gets one security notice: *Your
contact for the organization was changed to the new channel. If this
wasn't you, contact support.* No code is needed on the old channel. On
WhatsApp and Viber this notice needs an approved template bound to the
message key `contactChanged.notice.text` on the event; without one it is
refused with its reason, and the change itself still stands. It
isn't a second channel for the voter: it is a notice about the old channel
itself, and later codes and notices go only to the new one.

### Voter drawer and Logs

The voter drawer's **Messaging** section shows the voter's channel, the
masked number or Messenger Page, when consent was given, and two rows:

- **Last failure**: the latest message whose latest attempt failed, with
  its time and reason. A failed attempt that a later attempt of the same
  message retried or delivered is not shown.
- **Delivery not confirmed**: the latest message whose delivery is Unknown,
  or Unresolved after 24 hours. It is never shown as a failure.

Both need `messaging-config-read`; without it the drawer says so.

The voter's **Logs** dialog lists every message with its channel and its
**Status**: the message's current state from the message records with
`messaging-config-read`, otherwise the state when the entry was written,
marked as such. Under it is the reason: for a failure that isn't retried,
the failure and, after `; `, why it isn't retried, both in the
administrator's language.

### Send Notification

Send Notification has no channel choice: every notice goes to each voter's
own channel. The **Each voter's channel** table shows the channels the
event sends notices on and the account each sends from (the event's
account, else the tenant's default account; email and SMS without an
account use the deployment's own sender). An event without messaging
settings sends by email and SMS, as before. An event whose settings turn on
notices for no channel can't be sent to: the screen says so and nothing is
scheduled. Without `messaging-config-read` the screen says the channels
can't be shown; each voter still gets the notice on their own channel.
(How many of the selected voters use each channel is planned, not yet
available.) Each send is kept as an archived scheduled event that holds
what was sent, for its retries; it never appears in the event's Scheduled
Events list and can't be edited there.

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
same channel after 1 minute and then 5 minutes, up to three attempts in
all. After that the message stays failed and shows on the event tab, in
Logs and in the voter drawer. What is retried depends on where the message
comes from, because the platform never stores message content:

| Message | Retried? |
|---|---|
| Notifications from Send Notification | Yes, from what the administrator sent |
| Enrollment results (application accepted or rejected) | Yes, from the stored template |
| Codes | No: the voter presses **Resend code** |
| Keycloak notices (reset links, temporary credentials) | No: the voter asks again; the failure shows with its reason |
| Notifications whose template uses protected (encrypted) fields | No: *Can't be retried: contains protected fields* |
| Any message whose failure the provider reported more than 1 hour after the attempt | No: *Reported too late to retry* |

When a message isn't retried, its real failure reason stays visible, with
the reason it isn't retried next to it.

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
2. In the voter's Logs dialog, find the message's entry and its
   `message_id`. No screen shows the provider's message ID: an operator
   with database access reads `provider_message_id` from
   `sequent_backend.message` for that `message_id`, and looks it up in the
   provider's own console or delivery logs.
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
| Callback address must be https | Account | The deployment's public callback address isn't `https`; the URL is still shown, but providers such as Meta refuse it |
| Unsupported provider: replace this account | Account | The account uses a removed provider and sends nothing |
| Enter the credentials again for the new address | Account | The API address changed without new credentials |
| The provider address must start with https:// | Account | The API address isn't `https` |
| No contact channel | Message | The voter has no channel to send to |
| Channel not offered for notices, Channel not offered for codes | Message | The event or the voter's Post doesn't offer the voter's channel for that purpose |
| No content for the channel | Message | The notification has no text or template for the voter's channel |
| No sending account for the channel | Message | Neither the event nor the tenant has an account for the channel |
| Invalid destination for the channel | Message | The voter's number or address can't be used on the channel |
| Can't be retried: no stored template | Message | A failed message whose content can't be rebuilt; it stays failed |
| Can't be retried: contains protected fields | Message | The notification used protected fields, whose content is never stored |
| Reported too late to retry | Message | The provider reported the failure more than 1 hour after the attempt |
| Can't be retried: the message couldn't be prepared again | Message | The retry couldn't read its template, voter or event in time |
| Not the voter's channel | Message | A notice was asked for on a channel that isn't the voter's; nothing is sent on either |
| Generate the report credentials and create the Infobip subscription | Account | A Viber account without report credentials: its receipts are unavailable |
| The voter's channel changed: not retried | Message | The voter changed channel before the retry was due |
| The worker stopped before sending | Message | Never handed to the provider; retried like a temporary failure |
| The worker stopped before recording the outcome | Message | May have been sent; shown as Unknown and never resent |
| The code expired before it could be sent | Message | The code was too old to send |
| Delivery couldn't be confirmed | Message | Unknown for more than 24 hours (Unresolved) |

The Admin Portal shows every reason in the administrator's language.

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
