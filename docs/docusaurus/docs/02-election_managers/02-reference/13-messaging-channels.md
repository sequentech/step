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
(*Checked …*, or *Not checked yet*) and a short reason when one is missing:

- **Connected**: the provider accepted the credentials at the last check.
  An account never checked is not connected yet.
- **Ready for OTP** and **Ready for notices**: the purpose can be enabled
  for an event. A connected account is not ready while any prerequisite is
  missing, and the screen names it:
  - *Not checked for its current number, Page or sender*: the account's
    WhatsApp number or business account, Viber sender, Messenger Page, or
    SMS or email sender changed after its last check. Nothing that check
    said counts for the new one, so the account is ready for nothing until
    it is checked again (see [Changing the number or the Page](#changing-the-number-or-the-page)).
  - *Needs provider approval*: WhatsApp accounts start as pending until
    the organization's arrangement with Meta is approved (see
    [Going live](#going-live)). The check reads it from the number's status
    and display-name status; *Entered by an administrator* is used only
    while the check reports nothing.
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

Readiness is checked again **every time a message is sent**: a code or
notice through an account that isn't ready for it is not sent, and is
recorded as failed with *Not sent: the account isn't ready* and what is
missing. A notice from Send Notification or an enrollment result waits for
the account: it is retried once the account is ready again, if that
happens within an hour; after that it stays failed with *Can't be retried:
the message couldn't be prepared again*. Codes and Keycloak's own notices
are never retried (see [Delivery](#delivery)). **Send test message** is
the exception: it is how you try an account before it is ready.

### Connection checks

**Check connection** asks the provider about the account and stores the
result. Every account is also checked **once a day**, and soon after its
number, Page or sender changes; the drawer says when it was last checked.
An account whose check can't run is tried again 3 hours later. If the
account changes while its check runs, that check is discarded and **Check
connection** says *The account changed while it was checked: check it
again*.

The account's drawer shows what the provider reported at the last check,
or *Not reported*:

- the access token: valid, when it expires, and missing permissions
  (WhatsApp and Messenger);
- the app secret: accepted, refused or couldn't be checked;
- the WhatsApp number's status, display-name status, quality rating and
  messaging limit. A quality warning from Meta is shown as a warning,
  not as pending approval: the number still sends;
- the Viber partner login;
- the Messenger Page's `pages_messaging` permission, the app's subscription
  and the Get Started button;
- the callback address;
- each bound template's category and status per language;
- voters who changed their WhatsApp number (see
  [Changing the number or the Page](#changing-the-number-or-the-page)).

**Send test message** sends one message for a chosen purpose and
destination. *Accepted* means the provider took the message, not that it
was delivered.

### Credentials

Credentials are write-only. After saving, the account only shows when each
was last replaced. The account and the credentials typed with it are saved
together, in one step: either both are stored or neither is, and the drawer
shows the reason when the save is refused.

**Replacing a credential** that the connection check uses (an access
token, app secret, API key, or SMTP or AWS secret) checks the new one with
the provider **before** saving it. If it doesn't connect, nothing is saved,
the previous credential keeps sending, and the drawer says why, followed by
*The new credentials didn't connect: the previous ones are kept*. For
WhatsApp and Messenger, the new token must also be valid, not expired and
allowed to send, and Meta must accept the app secret. A credential entered
for the first time has nothing to keep: it is saved and then checked. A
replaced credential takes effect at once, with no restart.

Changing the address the credentials are sent to (the Graph API base URL,
the Infobip API base URL or the SMTP server) needs the credentials again,
because the stored ones would otherwise go to the new address. As soon as
the address changes, the drawer opens those credential fields and says
*Enter the credentials again for the new address*; it doesn't save until
they are typed. The address must start with `https://`.

### Sending limits

- **Messages per second**: the rate the provider allows the account. All
  of the platform's servers share it. Empty: messages are not throttled.
- **Reserved for OTPs per second**: a floor in that rate. Bulk sends stop
  at it, so codes always go first; with no codes being sent, a bulk send
  can use the full rate.
- **Codes sent at once**: how many codes, reset links and temporary
  passwords the account may have in flight at once on each server; empty
  means 8. Further ones wait their turn.
- **Allowed destinations**: the country calling codes the account may send
  to. Empty: any.

A code that can't get its turn within 5 seconds is not sent: it fails with
*Not sent: the account stayed at its sending limit for too long*, and the
voter presses **Resend code** after the usual wait. A provider that takes
too long to answer a code leaves it *Unknown*: *The provider didn't answer
in time: the message may have been sent*.

### Deleting an account

An account that an election event's messaging settings use can't be
deleted: the list says *The account was not deleted: Used by an election
event's messaging settings: remove it there first*. Choose another account
in those events first. Messages sent through a deleted account keep their
records; [monthly volumes](#monthly-volumes) show them as a deleted
account.

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

### Changing the number or the Page

Changing a WhatsApp account's number or business account, a Viber
account's sender, a Messenger account's Page, or an SMS or email account's
sender makes it a different sender:

- The drawer warns before saving: *Saving checks the account again in the
  background*. Until that check succeeds, the account is *Not checked for
  its current number, Page or sender*, and codes and notices through it
  fail as not ready; the test message is still sent.
- A new WhatsApp number needs its templates approved in that business
  account before it can send codes, and voters see a new chat.
- **Changing a Messenger Page disconnects the voters connected to the old
  Page**, because Page-scoped IDs belong to a Page. The drawer says how many
  voters are connected to the old Page and must connect again, across the
  election events that use the account. Those voters get codes again only
  after connecting Messenger again. Until then, codes and notices to them
  are not sent, with the reason *Not sent: the voter is connected to
  another Messenger Page*; nothing goes to another channel.
- **How they connect again.** At their next sign-in, after their
  password, the sign-in page explains that the Page changed and shows the
  link and QR code to the new Page. The voter taps Get Started there and
  enters the code; their new Messenger contact is saved. This happens only
  at sign-in, never in a password reset. A voter whose sign-in has no
  password step, or who forgot their password, can't reconnect alone:
  staff change their channel.
- Once saved, the events that use the account show voters the new Page or
  name on their pages.

When Meta reports that a voter changed their WhatsApp number, the account
lists the voters who had that number, with masked numbers, and says *A
voter changed their WhatsApp number: ask them to verify their contact
again*. Nothing is changed automatically: the voter changes the channel
from the voting portal (see [Changing the channel](#changing-the-channel)).

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

For each channel the tab selects an account and has separate **OTPs**
and **Notices** switches. A switch that can't be turned on is disabled with
the missing prerequisite, such as "Add an account in Settings > Messaging
first" or *Not checked for its current number, Page or sender*. Saving
checks every reference and prerequisite and names what is missing; nothing
is saved while anything is wrong.

**Delivery status** shows how each channel's messages are doing:

| Column | What it shows |
|---|---|
| Channel | The channel, with its icon. A channel the event doesn't use, but that has messages or voters, is shown too, marked *Not offered by this event* |
| Voters using it | Voters of the event whose channel this is |
| Sent, last 24 h | Messages whose latest attempt was in the last 24 hours, in any state, failed ones included |
| Queued, Accepted | Messages recorded but not yet handed over, and messages the provider took |
| Delivered | Messages the provider reported delivered, or *Unavailable* when the account has no receipts or no callback address |
| Failed | Messages whose latest attempt failed. Opens the event's Logs, filtered to this channel's failed messages |
| Unknown | Messages whose outcome is not confirmed yet |
| Unresolved | Unknown messages older than 24 hours (see [Unknown and Unresolved](#unknown-and-unresolved)) |

Counts are per message, by the state of its latest attempt: a message
retried twice counts once. Only *Sent, last 24 h* is limited in time; the
state columns count all of the event's messages. Logs filters on the same
thing: the **Status** filter matches each message's current state, so the
Failed link opens the same messages the count shows. Failures of voters
with no contact channel are never counted under a channel; they show on a
row of their own, *No contact channel*, whose failed count opens Logs
filtered to them (the Channel filter's *No contact channel* choice). When
the counts can't be loaded, the tab says so and shows dashes, never zero.

**Voters using it** is counted in the background, not each time the tab
opens, so large events don't slow it down. Under the table the tab says
*Counted at* and the time, or *Not counted yet*, and has a **Recount**
button. Voters are counted again after every save of the event's messaging
settings, when you press Recount, and regularly for events with messaging
settings (every 6 hours or so). A recount that takes too long or fails
says so, and the previous counts stay.

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

In an election event's **Voters** list, open **Columns** and select
**Messaging channel** to show each voter's channel. The column uses the same
channel rules as the voter details panel, including legacy email/SMS records,
and displays `-` when no usable channel is recorded. It shows only the channel
name and icon; contact details stay in the voter details panel. The column is
hidden by default and can be toggled with the other list columns.

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
  to, and a reference code to give support, such as `Q5KW-8RFX`, once the
  application is stored. The reference comes from the application's ID
  only. Staff find the application by typing the reference in the
  **Approvals** search box, in any case, with or without the dash; the
  search runs on the server, across the event's applications.
- **Sign-in**: the code goes only to the voter's channel, and the page
  names it.
- **Password reset**: the reset code or link goes to the voter's channel.
  The page always says *If an account exists, we sent the link to its
  contact*, without naming a channel. The reset code page is the same
  whether the account exists or not (the code step's setting
  `code-page-disclosure` is `NEUTRAL` on reset flows): it names no channel
  and shows no masked destination. Because that page shows no Messenger
  link either, a voter whose channel is Messenger can't get a reset code
  there and asks support. A refused or failed provider call shows the same
  neutral code page and starts the same resend timer; an unsent code cannot
  authenticate. An SMS provider that supports codes only cannot send a
  one-time reset link, and that refusal is recorded without generating a link.
- **Temporary credentials**: a new temporary password takes effect only
  when its notice was handed to the provider (queued, accepted, delivered
  or unknown). When the send is refused or fails, the password stays
  unchanged; the page stays neutral and the Keycloak event records the
  reason.

Masked destinations show the country code and the last three digits of a
number (*+966 ••• ••• 567*), the first two characters of an email address
and its domain (fewer when the part before the `@` is that short), or the
last three digits of a Messenger contact.

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
migration; the migration logs every voter it changes and why. This is
intended: when the channel such a voter chose can't be used (no number,
address or Messenger contact), the first verified channel that can is
taken. A channel the voter never verified is never taken.

### Changing the channel

A signed-in voter changes the channel from **Contact method** in
the voting portal's user menu; the item appears only when the event offers
at least one channel for codes (the portal reads the realm's public
`GET /realms/<realm>/messaging-channels`, which lists only channel names). The voter must have signed in within the
last 5 minutes, or is asked to sign in again. The page lists the Post's
channels, sends a code to the new channel, and when the code is verified
**replaces** the old channel completely. The required action
`message-channel-ra` must be registered and enabled in the event's realm;
realms that registered it as `messaging-app-otp-ra` keep working.
The change is offered only while every enabled message code step of the
realm's flows and their sub-flows sends to the voter's channel (courier
`CHOSEN`) or sends no code (`NONE`); otherwise it is refused with *A code
step doesn't send to the voter's channel*. The realm migration sets every
enabled message code step to `CHOSEN`, in every flow and sub-flow,
enrollment steps included; a step without settings gets the default
settings with that courier. A step set to send no code (`NONE`) is left
as it is, so it never starts sending; the migration logs it as *left
NONE: the step sends no code*. It logs each step it changes.

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

### Editing event template bindings

The event's Messaging tab shows a compact summary of each provider template
binding: purpose, language, message key, provider template and approval status.
Expand a binding to inspect or edit it. Opening another binding closes the
previous editor and retains its changes. Add opens the new binding immediately;
saving highlights and opens incomplete bindings. Save includes all bindings,
including those whose editors are closed.

### Send Notification

Send Notification has no channel choice: every notice goes to each voter's
own channel. The **Each voter's channel** table shows, for the voters the
notification is for:

| Column | What it shows |
|---|---|
| Channel | Each channel the voters use, in channel order, then voters with no contact channel |
| Voters | How many of them use it |
| Sends from | The event's account for the channel, else the tenant's default account; email and SMS without an account use the deployment's own sender |

Under the table: the voters in the audience, and how many will get the
notification. A row also says when its voters won't get it, and why:

- some voters' Posts don't offer the channel for notices (*N of them don't
  get it*), or none of them do (*Not sent* under Sends from);
- no account sends on the channel;
- the account the event names was deleted (*Deleted account*): those
  notices fail with *No sending account for the channel*, and never go out
  through another sender;
- the voters have no contact channel.

The counts for all voters, voters who voted and voters who didn't come
from the event's voter counts, with *Counted at* and **Recount**, as on the
event's Messaging tab. Selected voters are counted when you select them;
if that takes too long, the screen says so and shows no counts, and each
voter still gets the notification on their own channel.

An event without messaging settings sends by email and SMS, as before. An
event whose settings turn on notices for no channel can't be sent to: the
screen says so and nothing is scheduled. Without `messaging-config-read`
the screen says the channels can't be shown; each voter still gets the
notice on their own channel. Each send is kept as an archived scheduled
event that holds what was sent, for its retries; it never appears in the
event's Scheduled Events list and can't be edited there.

> Each voter gets one message on their own channel. If delivery fails, it is
> retried on that channel and the failure shows on the event's Messaging
> tab. Unconfirmed delivery is shown as Unknown.

Sending to selected voters with no voter chosen is refused.

The channel sections start collapsed, as in the template editor. Expand a
channel to review its content or make changes for this notification. Closing a
section keeps its content and edits; it does not turn off delivery on that
channel. Email, SMS and Messenger text can be edited after choosing a template,
without changing the saved template. WhatsApp and Viber wording appears as a
plain preview with a notice explaining that provider-approved content cannot
be changed here. The send drawer has no provider-template or language inputs:
those values come from the saved template, or the event's binding when absent.
Choose another saved template to use different approved content.

WhatsApp accepts language codes and regional locale codes. Use the exact code
of the approved template, such as `en` or `en_US`; do not replace one with the
other just to match an example. The demo's `en` defaults are supported by its
approved templates. See [Meta's language object reference](https://whatsapp.github.io/WhatsApp-Nodejs-SDK/api-reference/types/language_object/).

The form checks that the notification has content for the audience's channels
before scheduling it. For WhatsApp or Viber, choose a saved message template
with content and an approved provider template or event binding. Email and SMS
text alone cannot be delivered to a WhatsApp voter. The success notice confirms
scheduling; delivery results appear in the event's Messaging tab and Logs.

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
| Not sent: the voter is connected to another Messenger Page | Message | The account's Messenger Page changed after the voter connected; the voter connects again at their next sign-in (see [Changing the number or the Page](#changing-the-number-or-the-page)). Nothing was handed to the provider |
| Generate the report credentials and create the Infobip subscription | Account | A Viber account without report credentials: its receipts are unavailable |
| The voter's channel changed: not retried | Message | The voter changed channel before the retry was due |
| The worker stopped before sending | Message | Never handed to the provider; retried like a temporary failure |
| The worker stopped before recording the outcome | Message | May have been sent; shown as Unknown and never resent |
| The code expired before it could be sent | Message | The code was too old to send |
| Delivery couldn't be confirmed | Message | Unknown for more than 24 hours (Unresolved) |
| Not sent: the account isn't ready (…) | Message | The account wasn't ready for the purpose when the message was due; what is missing is named in brackets. Nothing was handed to the provider |
| Not sent: the account stayed at its sending limit for too long | Message | A code, reset link or temporary password waited more than 5 seconds for the account's sending limit; nothing was handed to the provider |
| The provider didn't answer in time: the message may have been sent | Message | The provider took too long to answer a code or Keycloak notice; shown as Unknown |
| The new credentials didn't connect: the previous ones are kept | Account | A replaced credential failed its check; nothing was saved |
| Used by an election event's messaging settings: remove it there first | Account | The account can't be deleted while an event uses it |
| The account changed while it was checked: check it again | Account | Its number, Page, sender or credentials changed during the check, so the result was discarded |
| A voter changed their WhatsApp number: ask them to verify their contact again | Account | Meta reported a voter's new WhatsApp number |

The Admin Portal shows every reason in the administrator's language.

## Records

Every attempt is recorded with the channel, account, purpose, masked
recipient, recipient country, provider message ID, state and the billing
data the provider reports. The electoral log records each attempt's
channel, account, purpose and outcome with the masked recipient. Codes,
message bodies, credentials and messages voters send to the accounts are
never recorded.

### Monthly volumes

**Settings > Messaging** shows **Monthly volumes** under the account list,
for a month you choose (months in UTC), to reconcile the providers'
invoices. Viewing it needs `messaging-account-read` or
`messaging-config-read`. It counts **attempts handed to a provider**, each
in the month it was made, so a retry counts again, as providers bill it:

| Column | What it shows |
|---|---|
| Channel | The channel |
| Sent from | The account. *Deleted account* for WhatsApp, Viber and Messenger attempts whose account was deleted since; for email and SMS, *Deployment sender or a deleted account*, because the records can't tell them apart |
| Destination country | The recipient's country, for phone numbers |
| Billing category | Authentication, Utility, Service, Transactional or Email: the one expected when sending, or the one the provider reported |
| Billable | What the provider reported, or *Not reported* |
| Attempts | How many |

Messages never handed to a provider (refused, waiting for the sending
limit, or a code that expired first) aren't counted. No prices are shown.

## Sent messages (development stacks only)

On a development or demonstration stack whose fake provider is turned on,
**Settings > Messaging** has two tabs, **Accounts** and **Sent messages**.
Sent messages shows what the fake provider received, as the voters' apps
would show it; none of it reached a real voter, and codes are visible so
that you can sign in as a test voter. Choose **All** or a channel; the
choice is kept in the address as `?channel=` (`all`, `whatsapp`, `viber`,
`messenger`, `sms` or `email`), so a link opens the same view. The
Messenger view has a **Get Started** box: paste the link from the
enrollment page and press Get Started to stand in for the voter's tap.
Real deployments never show this tab. See
[Messaging development](../../07-developers/14-messaging/03-messaging-development.md#sent-messages-viewer).

## Going live

Every provider agreement is applied by configuration, and none needs a new
version of the platform for a supported provider. The deployment
prerequisites are listed in
[Messaging deployment](../../07-developers/14-messaging/02-messaging-deployment.md#deployment-prerequisites).
In short:

- **WhatsApp**: the organization holds the Meta business portfolio, the
  WhatsApp Business Account and the number, through a Solution Partner.
  Once Meta approves the arrangement and the templates, enter the
  credentials and run **Check connection**: the check reads the approval
  from the number's status. A Solution
  Partner that serves the Cloud API from its own address is entered with
  its API base URL.
- **Viber**: choose the partner. Infobip is built in; another partner needs
  an adapter. Enter the partner's credentials and approved templates, and
  create the report subscription.
- **Messenger outside the 24-hour window**: once Meta approves utility
  templates for the Page, bind them on the event.
- Which channels each Post offers, the consent wording and how long message
  records are kept.
