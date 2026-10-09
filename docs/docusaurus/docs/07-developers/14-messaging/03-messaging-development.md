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
  the network;
- `MESSAGING_RELAY_SECRET`: a development-only value.

Meta's handshake through the component, from the devcontainer:

```sh
curl "http://messaging-webhooks:8410/webhooks/meta/<key>?hub.mode=subscribe&hub.verify_token=<token>&hub.challenge=42"
```

It answers `42` as plain text when the key and token match an account.

<!-- DRAFT (W2): the next two sections describe the fake provider routes and
the Sent messages viewer, which are being built. Re-check them against the
code before release. -->

## Fake provider routes (draft)

Harvest mounts `/fake-provider/*` routes only when
`MESSAGING_FAKE_PROVIDER_ROUTES=ENABLED` (`DISABLED` by default). That is
all the variable does: no other code reads it, and real deployments leave
it unset. The senders reach the routes only because the development `.env`
points the endpoint variables at them:

```sh
MESSAGING_FAKE_PROVIDER_ROUTES=ENABLED
MESSAGING_META_GRAPH_URL=http://harvest:${HARVEST_PORT}/fake-provider/meta
MESSAGING_INFOBIP_URL=http://harvest:${HARVEST_PORT}/fake-provider/infobip
MESSAGING_AWS_ENDPOINT_URL=http://harvest:${HARVEST_PORT}/fake-provider/aws
```

| Provider | Send | Checks (answered as connected and ready) |
|---|---|---|
| WhatsApp Cloud | `POST /fake-provider/meta/<version>/<phone_number_id>/messages` | `debug_token`, phone number fields, `message_templates` (bound templates approved) |
| Messenger | `POST /fake-provider/meta/<version>/<page_id>/messages` | `debug_token`, Page fields, `messenger_profile` |
| Viber (Infobip) | `POST /fake-provider/infobip/viber/2/messages` | `GET /account/1/balance` |
| SMS (SNS) | `POST /fake-provider/aws/` (`Action=Publish`) | `GetSMSSandboxAccountStatus` |
| Email (SES v2) | `POST /fake-provider/aws/v2/email/outbound-emails` | `GET /v2/email/account` |

Every message is accepted and kept in harvest's memory (the last 1000, lost
on restart). About a second later the routes post a DELIVERED report the way
the real provider would, so it takes the production path through the
webhook component, `/messaging/callbacks` and verification:

- WhatsApp and Messenger: a Meta status event signed with
  `X-Hub-Signature-256` to `<MESSAGING_WEBHOOKS_PUBLIC_URL>/webhooks/meta/<key>`.
- Viber: an Infobip report with the account's Basic authentication to
  `<MESSAGING_WEBHOOKS_PUBLIC_URL>/webhooks/viber/<key>`.
- SMS and email get no report, as in production without receipts.

There are no failure or behaviour controls. Failure handling is tested with
the loopback provider in the test suites, not with these routes.

`POST /fake-provider/messages` returns the kept messages, codes included.
`POST /fake-provider/meta/messenger/get-started` takes `{link}`, the
`m.me` link the enrollment page shows, and posts Meta's signed Get Started
postback for it through the component, standing in for the voter's tap. The
Hasura actions `messaging_fake_provider_messages` and
`messaging_fake_provider_get_started` (permission `messaging-account-write`)
point at them, and fail like any action without a handler when the routes
aren't mounted.

## Sent messages viewer (draft)

**Settings > Messaging > Sent messages** shows what the fake provider
received. The tab appears only when `messaging_fake_provider_messages`
answers. It has an **All** table (time, channel, from, to, message) and a
view per channel in the app's style, chosen with `?channel=`; it refreshes
every 5 seconds. The Messenger view has a **Get Started** box: paste the
link from the enrollment page, press Get Started, and the code arrives in
the chat. Email HTML is shown in a sandboxed `iframe`.

The `step-dev scenario` for messaging creates accounts on the fake
provider, an event configuration and a second organization. Run it through
devenv: `devenv shell -- python3 -m scripts.dev.scenario ...`.

## Tests

- `cargo test -p messaging`: adapters against a loopback HTTP peer, webhook
  verification, the relay envelope and signature, decisions and rates.
- `cargo test -p messaging-webhooks`: the route list, unknown keys (404,
  nothing forwarded), the body limit (413), the header allowlist, byte-exact
  bodies, handshake passthrough, and 503 when harvest is down.
- harvest route tests: `/messaging/callbacks` with missing, wrong or stale
  relay signatures (401, nothing processed), Meta, SNS and Viber checks on
  relayed bytes, and forged or wrong-tenant tokens on `/messages/*`.
- `cargo test -p windmill --test postgres_messaging --test
  postgres_messaging_dispatch` against PostgreSQL (see the
  [windmill test guide](../08-windmill/test-coverage.md)): persistence,
  replays, same-channel retries, never-sent and maybe-sent, the sweeps, lost
  answers, signed reports and Messenger links. Set `MASTER_SECRET` as well.
- `mvn -f packages/keycloak-extensions/pom.xml test`: the SPI, the one
  channel at sign-in, attempts across resends, contact change, reset and
  Messenger linking.
