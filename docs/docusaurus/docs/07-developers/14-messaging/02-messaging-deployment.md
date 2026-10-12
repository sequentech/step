---
id: messaging_deployment
title: Messaging Deployment
sidebar_label: Deployment
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

What a deployment needs for WhatsApp, Viber, Messenger, SMS and email. The
design is in [Messaging Architecture](01-messaging-architecture.md).

## The one public endpoint

The webhook component, `messaging-webhooks`, is the **only new inbound
endpoint** messaging adds, and the only one providers call. Expose its
`/webhooks/` routes over HTTPS on a public host name; nothing else of the
backend becomes reachable.

- **Harvest is never public.** Don't add an ingress, proxy path or tunnel to
  harvest for messaging. Providers reach harvest only through the component,
  which relays to `POST /messaging/callbacks` on the internal network.
- The component's `GET /healthz` is for the orchestrator's liveness probe;
  it doesn't need to be public.
- The component needs outbound access to harvest only. Harvest and windmill
  need outbound access to the providers' APIs.

| Service | Variables |
|---|---|
| `messaging-webhooks` | `MESSAGING_RELAY_SECRET`, `HARVEST_DOMAIN`, `MESSAGING_WEBHOOKS_MAX_BODY_BYTES` (optional), `MESSAGING_WEBHOOKS_MAX_IN_FLIGHT` (optional), `ROCKET_ADDRESS`, `ROCKET_PORT` |
| harvest | `MESSAGING_RELAY_SECRET`, `MESSAGING_WEBHOOKS_PUBLIC_URL`, `MESSAGING_CALLBACK_CONNECTIONS` (optional), and the endpoint variables if used |
| windmill | `MESSAGING_WEBHOOKS_PUBLIC_URL`, and the endpoint variables if used |
| beat | `MESSAGING_WEBHOOKS_PUBLIC_URL`, and the endpoint variables if used |

- `MESSAGING_WEBHOOKS_PUBLIC_URL` is the public base address of the
  component, such as `https://callbacks.example.org`. Give harvest,
  windmill and beat **the same value**: harvest builds each account's
  callback URL from it and fills the callback status of **Check
  connection**, and windmill fills it in the daily account check, so two
  values would make the account's status change with whichever ran last.
  The component itself doesn't read it. Without it, accounts show
  "Callback address not configured", delivery shows as unavailable and
  Messenger is not ready for codes. It must be `https`: otherwise accounts
  show "Callback address must be https" next to the URL (the development
  stack's `http://` address is the only expected exception).
- `MESSAGING_RELAY_SECRET` is a random value of at least 32 bytes
  (`openssl rand -hex 32`), shared by the component and harvest only.
  Store it like any other deployment secret. The development stack's
  committed value is refused unless `MESSAGING_RELAY_SECRET_POLICY` is
  `ALLOW_DEV_SECRET`, which only the development compose file sets: leave
  the policy unset. Without a usable secret the component doesn't start and
  harvest answers relayed callbacks with 503. Rotating it means changing
  both together; callbacks during the change get 503 and the providers
  retry them.
- The endpoint variables (`MESSAGING_META_GRAPH_URL`,
  `MESSAGING_INFOBIP_URL`, `MESSAGING_AWS_ENDPOINT_URL`) stay unset in a
  normal deployment. When one is set, give harvest, windmill and beat the
  same value.
- `MESSAGING_FAKE_PROVIDER_ROUTES` is **never set** in a real deployment.
  It mounts development-only routes in harvest that answer as the
  providers (see
  [Fake provider routes](03-messaging-development.md#fake-provider-routes));
  unset, those routes don't exist.

- `MESSAGING_WEBHOOKS_MAX_IN_FLIGHT` (16 by default) and
  `MESSAGING_CALLBACK_CONNECTIONS` (4 by default) bound what a burst of
  callbacks can hold: requests to harvest per component process, and
  database connections per harvest process. Callbacks beyond them get 503
  and the providers send them again, so codes and notices keep the rest of
  harvest's pool. Raise them only together with harvest's pool size.

The component is built like harvest (`Dockerfile.cargo-packages` and the
product images) and runs with its own command. It is stateless: run as many
replicas as needed behind the public host.

## Provider setup

Each account's drawer in **Settings > Messaging** shows its callback URL:
`<MESSAGING_WEBHOOKS_PUBLIC_URL>/webhooks/<meta|viber|aws>/<key>`. The key is
generated per account and identifies it; there is nothing else to
configure on the platform side.

- **WhatsApp**: in the Meta app, set the WhatsApp webhook's callback URL and
  verify token (generated on the account, shown once) and subscribe the
  `messages` field, which carries the statuses.
- **Messenger**: in the Meta app, set the Page webhook's callback URL and
  verify token, subscribe the app to the Page for messages, postbacks,
  referrals and deliveries, set the Page's Get Started button, and grant
  `pages_messaging`.
- **Viber (Infobip)**: create a subscription for Viber delivery reports
  with the callback URL and Basic authentication, using the report username
  and password the account generated (shown once).
- **Email (SES)**: send the identity's delivery, bounce and complaint
  notifications to an SNS topic, subscribe the callback URL to it over
  HTTPS, and enter the topic ARN on the account.
- **SMS (SNS)** and **SMTP** have no receipts and need no callback.

Run **Check connection** on each account afterwards; it reports what is
still missing. After that, every account is checked again once a day, and
its readiness gates every send through it.

## Sending limits

Set **Messages per second** on each account to the rate the provider
allows. All harvest and windmill processes share that rate through one
bucket per account in the database. **Reserved for OTPs per second** keeps
part of it for codes during bulk sends, and **Codes sent at once** (8 by
default) limits the sends Keycloak asks for that one harvest process has
in flight for the account. An account without a rate isn't throttled.

A send Keycloak asks for waits at most 5 s for its account's capacity, and
harvest answers it within 8 s. Keep the Keycloak sender's
`--spi-message-sender-harvest-timeout-seconds` at its default (10) or
above.

## Rolling back

A rollback to the previous release runs the migration's `down.sql`
**before** the previous binaries are deployed. The previous binaries can't
read message rows without a channel (the "No contact channel" rows), so
code rolled back on the new schema fails on them.

`down.sql` reports each rewrite with `RAISE NOTICE`; keep its output with
the rollback record:

- the message rows without a channel it relabels `EMAIL` (the previous
  release then counts them as email failures);
- the Messenger links without a link word it gives their reference digest;
- the event configurations it drops. Every configuration save since this
  release also writes the election event annotation the previous release
  reads, so each event goes back to its last saved configuration.

The inbound messages this release deleted aren't restored.

## Deployment prerequisites

These are outside the platform and must be settled before go-live.

| | Prerequisite | Owner |
|---|---|---|
| D1 | **WhatsApp ownership and onboarding.** The organization owns the Meta business portfolio, the WhatsApp Business Account and the number, onboarded through a Solution Partner; Meta approves the account and its templates. Get written confirmation that the whole operating arrangement is permitted, and the provider's onboarding lead time and approved capacity. Meta doesn't let a voting-system vendor hold the account | Organization, Meta, Solution Partner |
| D2 | **The Viber partner.** One partner means one adapter; Infobip is built in. The Viber business account belongs to the organization. Confirm the report subscription with Basic authentication on the partner's test account | Organization |
| D3 | **Channels per Post.** Approve the channels and purposes each Post offers, using provider restrictions and test sends. Start from ready, approved channels, not all five | Organization |
| D4 | **Consent and retention.** The consent wording, and how long message records are kept | Organization (data privacy) |
| D5 | **Messenger utility templates.** Meta's approval of the Page's utility templates, which notices use after the 24-hour window. A code outside the window needs a fresh Get Started | Meta |
| D6 | **Communications log.** An owner for the log screen, conversation view and exports. The platform provides the records, Logs and the voter drawer | Product |
| D7 | **Demonstration.** Any demonstration uses the organization's own WhatsApp account, Viber through a partner test account, or an offline method the organization approves | Organization |

Approvals, quota increases and partner onboarding take calendar time and
are not engineering guarantees.
