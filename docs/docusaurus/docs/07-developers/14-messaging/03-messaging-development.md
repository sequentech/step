---
id: messaging_development
title: Messaging Development and Tests
sidebar_label: Development and tests
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

How to run messaging on the development stack and test it. The design is in
[Messaging Architecture](01-messaging-architecture.md).

## Local stack

The compose service `messaging-webhooks` (profiles `full` and `base`) runs
the webhook component with `cargo run`, like harvest, on
`MESSAGING_WEBHOOKS_PORT` (8410). The development `.env` sets:

- `MESSAGING_WEBHOOKS_PUBLIC_URL=http://messaging-webhooks:8410`: the
  compose host name, because callbacks in development are posted from inside
  the network (accounts show "Callback address must be https" next to it,
  as expected);
- `MESSAGING_RELAY_SECRET`: a development-only value, which the component
  and harvest accept only because the development compose file sets
  `MESSAGING_RELAY_SECRET_POLICY=ALLOW_DEV_SECRET`;
- the [fake provider routes](#fake-provider-routes) switch and endpoint
  variables.

Meta's handshake through the component, from the devcontainer:

```sh
curl "http://messaging-webhooks:8410/webhooks/meta/<key>?hub.mode=subscribe&hub.verify_token=<token>&hub.challenge=42"
```

It answers `42` as plain text when the key and token match an account.

## Fake provider routes

On the development stack nothing reaches a real voter: harvest answers as
the providers. It mounts `/fake-provider/*` routes only when
`MESSAGING_FAKE_PROVIDER_ROUTES=ENABLED` (`DISABLED` by default; an
unknown value is `DISABLED` and logged). That is all the variable does:
nothing else reads it, and nothing in the backend, Keycloak or the Admin
Portal branches on it. Real deployments leave it unset. The senders reach
the routes only because the development `.env` points the endpoint
variables at them, as it could point them at any provider:

```sh
MESSAGING_FAKE_PROVIDER_ROUTES=ENABLED
MESSAGING_META_GRAPH_URL=http://harvest:${HARVEST_PORT}/fake-provider/meta
MESSAGING_INFOBIP_URL=http://harvest:${HARVEST_PORT}/fake-provider/infobip
MESSAGING_AWS_ENDPOINT_URL=http://harvest:${HARVEST_PORT}/fake-provider/aws
```

The development compose file passes the switch to harvest, and the
endpoint variables to harvest, windmill and beat.

| Provider | Send | Checks (answered as connected and ready) |
|---|---|---|
| WhatsApp Cloud | `POST /fake-provider/meta/<version>/<phone_number_id>/messages` | `debug_token`, the number's fields (status, name status, quality, limit), `message_templates` |
| Viber (Infobip) | `POST /fake-provider/infobip/viber/2/messages` | `GET /fake-provider/infobip/account/1/balance`; reconciliation reads `GET /fake-provider/infobip/viber/2/logs` |
| Messenger | `POST /fake-provider/meta/<version>/<page_id>/messages` | `debug_token`, Page fields, `subscribed_apps`, `messenger_profile`, `message_templates` |
| SMS (SNS) | `POST /fake-provider/aws/` (`Action=Publish`, XML answer) | `GetSMSSandboxAccountStatus` |
| Email (SES v2) | `POST /fake-provider/aws/v2/email/outbound-emails` | `GET /fake-provider/aws/v2/email/account` |

The fake reports the templates `step_code` (authentication) and
`step_notice` (utility) as approved in en, en_US, es, ca, eu, fr, gl, nl
and tl; bind those names on the event.

**Credentials are required, as at the provider.** A send is accepted only
from an account that uses that sender **and** presents its own credential:
the Meta bearer token (`ACCESS_TOKEN`), the Infobip `App` key (`API_KEY`),
or an AWS Signature Version 4 made with the account's own
`AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` (the signature is checked
as AWS does: the signed headers, the body digest and a time within 15
minutes). Anything else gets the provider's own 401 or 403 and is not
kept; an AWS request signed with another secret gets
`SignatureDoesNotMatch`. The check routes need a
credential present but accept any value, because candidate credentials are
checked before they are stored. So:

- Development AWS accounts (SMS and email) need their **own** access keys.
  An account without them signs with the process's environment
  credentials, which match no account, and its sends are refused.
- A hand-made account needs its own token or key, which the adapters send
  anyway.

**Reports take the production path.** About 3 seconds after accepting a
WhatsApp, Viber or Messenger message, the routes post a DELIVERED report
for it to the webhook component, as the provider would:

- WhatsApp and Messenger: a Meta status event signed with the account's
  app secret (`X-Hub-Signature-256`) to
  `<MESSAGING_WEBHOOKS_PUBLIC_URL>/webhooks/meta/<key>`;
- Viber: an Infobip report with the account's report Basic authentication
  to `<MESSAGING_WEBHOOKS_PUBLIC_URL>/webhooks/viber/<key>`.

It then goes through `/messaging/callbacks`, verification and the ledger;
the backend can't tell it came from the fake. SMS (SNS) and email (SES)
get no report, as in production without receipts: SES reports are signed
by SNS, and verification trusts only AWS certificates. Each kept message
records its report's result: answered (with the component's HTTP status),
pending, not sent (with the reason, such as "Several accounts use this
sender" or "Callback address not configured"), unreachable, or no receipts
on this channel. A kept message always has at least one account: a send no
account authenticates is refused before anything is kept. An account without its app secret or report credentials
shows its reports refused, and its messages stay *Accepted*.

There are no failure or behaviour controls: every authenticated message is
accepted. Failure handling is tested with the loopback provider in the test
suites, not with these routes.

**What is kept.** Each message is kept in harvest's memory, lost on
restart, with no table: its time, channel, provider, sender, recipient and
content (text, template and parameters, or email subject, text and HTML),
codes included by design; codes are never logged. A message belongs to the
tenant whose account matched its sender and credential: the list keeps the
last 1000 per tenant (20,000 and 64 MiB in all), and a message more than
one tenant could claim is shown to none. Each content field keeps its first
64 KiB and says when it was cut.

**Reading them.** Three Hasura actions, all `messaging-account-write`,
point at the routes. When the routes aren't mounted, they fail like any
action without a handler.

| Action | Route | What it does |
|---|---|---|
| `messaging_fake_provider_messages` (query) | `POST /fake-provider/messages` | The caller's tenant's messages, without email HTML (`has_html` says there is one) |
| `messaging_fake_provider_message(id)` (query) | `POST /fake-provider/message` | One message, with its email HTML; "The message is no longer in the list" when it was dropped or is another tenant's |
| `messaging_fake_provider_get_started(link)` (mutation) | `POST /fake-provider/meta/messenger/get-started` | Stands in for the voter's Get Started tap (below) |

**Messenger Get Started.** The development stack has no Messenger app. The
Get Started route takes the `m.me` link the enrollment page shows (or its
QR code encodes), finds the tenant's Messenger account with that Page
username, and posts Meta's signed first-interaction postback (the link's
`ref`, a made-up Page-scoped ID that is stable per link) through the
webhook component. The backend binds the link and sends the code, which
appears in the Messenger chat of the viewer below; a second Get Started on
the same link sends no new code. The route returns the component's answer,
or a refusal with its reason ("Paste the m.me link from the enrollment
page", "The link has no ref: copy the whole link", "No Messenger account of
this organization uses this Page").

## Sent messages viewer

**Settings > Messaging > Sent messages** shows what the fake provider
received, for administrators with `messaging-account-write`. The tab
appears only once `messaging_fake_provider_messages` answers; where the
routes aren't mounted the action fails and Settings > Messaging shows only
the accounts, with no tabs. If a later refresh fails, the tab stays with a
warning and the last list.

- **Modes:** All · WhatsApp · Viber · Messenger · SMS · Email, kept in the
  URL as `?channel=all|whatsapp|viber|messenger|sms|email` (any other value
  is All). A link with `?channel=` opens the tab in that mode; choosing
  Accounts removes it. Settings' own `tabIndex` depends on the user's
  permissions, so a deep link is, for example,
  `/settings?tabIndex=2&channel=whatsapp` for a user who sees all the
  Settings tabs.
- **All** is a table: Time, Channel, From, To, Message, Delivery report.
  **Show more** opens the complete captured message below its row; **Show less**
  collapses it. Text wraps with its original line breaks, template parameters
  and codes are visible, and emails use the same sandboxed reader as Email mode.
  Expanded messages stay open across automatic refreshes.
- **Channel modes** show a phone frame with one chat per sender and
  recipient, in our own colours and icons (never the apps' logos):
  WhatsApp shows the code with a **Copy code** button; the Messenger view
  has a **Get Started** box (paste the enrollment page's link, press Get
  Started, and the code arrives in the chat); Email is an inbox with a
  reading pane.
- Email HTML is read only for the open email, and shown in an `iframe`
  with `sandbox=""` and a content security policy that blocks remote
  loads; `<meta http-equiv="refresh">` and `<base>` are removed first.
- It refreshes every 5 seconds while open, and has a Refresh button.
  Recipients are shown unmasked: they are the development stack's own
  synthetic destinations.

## Scenarios

Two `step-dev scenario` names set messaging up on the running stack. Run
them in the devenv shell (`scripts/dev/step-dev` needs Python 3.11 or
later):

```sh
devenv shell -- scripts/dev/step-dev scenario up messaging
devenv shell -- scripts/dev/step-dev scenario up messaging-second-organization
```

- **`messaging`** imports the event with the development realm's
  single-channel sign-in, creates the tenant's accounts on the fake
  provider routes for all five channels, checks them, configures the
  event's messaging and opens online voting. Codes are read in Sent
  messages.
- **`messaging-second-organization`** creates a real second tenant,
  `staff-association` ("Staff Association", English and Spanish, Spanish
  time zones), the way an operator does (`insertTenant`), enrolls its
  administrator, and gives it its own accounts on all five channels, with
  other senders, purposes and labels, and channels restricted per
  election. It shows every messaging screen for an organization other than
  the first.

Email uses SES on the fake routes: SMTP accounts need a real SMTP server,
and the development stack has none. A rerun reuses what exists: it sets
missing credentials again, keeps settings and default accounts the tenant
already has (and says so), and only adds what is missing. `scenario reset`
deletes the event only; the accounts, and the second tenant, stay.

Both scenarios also seed **Demo notification** in Settings > Templates, with
content for all five channels and `step_notice` as the approved provider template.
Existing templates are kept. To send a manual demo notice, select a voter,
open Send Notification, choose Demo notification and send. Sent messages shows
what the local provider received; a notice refused before delivery appears in
the event's Messaging failure count and Logs instead.

## Capacity benchmark

`step-dev bench messaging-otp` measures code latency (p50, p95, p99) while
a bulk send saturates the same account, with harvest, the bulk sender
(windmill's dispatch path) and a loopback provider as separate processes
built from the checkout. It sets the account's limits for the run and
restores them, empties the bucket before each window, measures a
codes-alone window and then a window during the bulk send, and deletes its
ledger rows unless `--keep-rows`. Use a development tenant and a WhatsApp,
Viber or Messenger account harvest chooses for codes:

```sh
devenv shell -- bash -c 'scripts/dev/step-dev bench messaging-otp \
  --label after --checkout . --tenant-id "$TENANT_ID" --account-id "$ACCOUNT_ID" \
  --election-event-id "$EVENT_ID" --messages-per-second 50 \
  --otp-reserved-per-second 10 --otp-rate 5 --duration 60 \
  --bulk-workers 16 --provider-latency-ms 150'
```

The result is a `step-bench/1` file under `~/.cache/step-bench/results`
(or `--output-dir`). Codes past the 5 s budget show as `FAILED` with the
capacity reason.

## Tests

- `cargo test -p messaging`: adapters against a loopback HTTP peer, webhook
  verification, the relay envelope and signature, decisions and rates.
- `cargo test -p messaging-webhooks`: the route list, unknown keys (404,
  nothing forwarded), the body limit (413), the header allowlist, byte-exact
  bodies, handshake passthrough, and 503 when harvest is down.
- harvest route tests (`cargo test -p harvest --bin harvest -- messaging
  otp_capacity fake_provider`): `/messaging/callbacks` with missing, wrong
  or stale relay signatures (401, nothing processed), Meta, SNS and Viber
  checks on relayed bytes, forged or wrong-tenant tokens on `/messages/*`,
  the send gate for Keycloak's sends, a saturated account answered
  `FAILED` with `CAPACITY_BUSY` before Keycloak's timeout, a late provider
  left `UNKNOWN`, Get Started answered before its code is sent, the
  candidate and Page-change routes, and the fake provider routes end to
  end through the real webhook component (sends, credentials, tenants,
  reports into the ledger, refused reports, Get Started). The fake
  provider routes are mounted only by their switch
  (`request_boundaries`).
- `cargo test -p windmill --test postgres_messaging --test
  postgres_messaging_dispatch --test postgres_messaging_retry --test
  postgres_messaging_config --test postgres_messaging_rate_bucket --test
  postgres_messaging_accounts --test postgres_messaging_account_checks
  --test postgres_messaging_insights --test postgres_messaging_migration`
  against PostgreSQL (see the
  [windmill test guide](../08-windmill/test-coverage.md)): persistence,
  replays, same-channel retries and what is retried by origin, never-sent
  and maybe-sent, the sweeps, lost answers, signed reports, Messenger links,
  the event configuration table and the account rules (credentials follow
  their address, `https` only); the shared bucket under concurrent takers
  on separate pools, the reserve, deadlines and the send gate on every
  path; candidate credentials, identity-stamped and stale checks, delete
  against a config save, the daily pass; voter counts equal to
  `voter_channel` and the recount, volumes of dispatched attempts. Set
  `MASTER_SECRET` as well.
- `python3 -m unittest scripts.dev.test_scenario
  scripts.dev.bench.test_messaging_otp` in the devenv shell: the messaging
  scenarios, the second tenant and the benchmark's arguments.
- Admin Portal (`packages/admin-portal`): `yarn jest src/services
  src/resources/Settings src/resources/ElectionEvent src/resources/User
  src/components src/translations`. `src/services/messagingReasons.test.ts`
  reads the backend's reason strings from the Rust sources and fails until
  every one is translated in all 8 languages;
  `src/services/messagingClientNames.test.ts` fails when product messaging
  code (TypeScript, Rust or Java) names a client. The stories under
  `src/screens/SecondOrganization/` render every messaging screen for the
  second organization, and `src/resources/Settings/SentMessages/` has a
  story per viewer mode.
- `mvn -f packages/keycloak-extensions/pom.xml test`: the SPI, the one
  channel at sign-in, attempts across resends, contact change, reset and
  Messenger linking.
