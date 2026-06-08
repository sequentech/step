---
id: smart_link_integration_guide
title: Smart Link SSO Integration
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Smart Link SSO Integration Guide

## Overview

This guide is for **election managers and third-party organizations** who want
to send voters that are already authenticated in their own system straight into
a Sequent election, without asking them to log in again.

Smart Link is a Single Sign-On (SSO) mechanism based on a keyed
[HMAC](https://en.wikipedia.org/wiki/HMAC) authentication token. Your backend
mints a short-lived token, signed with a secret shared only between your servers
and Sequent, and the voter follows a link that carries that token. Sequent
verifies the token and logs the voter in.

:::note Other SSO methods
Sequent also supports SAML 2.0 IdP-initiated SSO and OpenID Connect. See the
[IdP-Initiated SSO Integration](./idp_initiated_sso_integration_guide.md) guide
if you already run a SAML identity provider.
:::

### Authentication, not authorization

Your external application performs **authentication** (it decides _who_ the voter
is). Sequent performs **authorization** (it decides whether that voter is _allowed_
to vote, by matching them against the election census). This means the voter must
already exist in the election census in Sequent; the Smart Link does not, by
default, create voters.

## How it works

```mermaid
sequenceDiagram
    participant Voter as Voter Browser
    participant App as Your Application (frontend)
    participant Backend as Your Backend (knows the secret)
    participant KC as Sequent Keycloak
    participant Portal as Sequent Voting Portal

    Voter->>App: 1. Click "Vote"
    App->>Backend: 2. Request a Smart Link (authenticated)
    Backend->>Backend: 3. Build message + HMAC-SHA256(secret)
    Backend-->>App: 4. Return the Smart Link URL
    App->>KC: 5. Open /realms/{realm}/smart-link/login?auth-token=...
    KC->>KC: 6. Verify HMAC, time window, event, census user
    KC-->>Portal: 7. Redirect with an OIDC session (logged in)
```

The single voter interaction (clicking "Vote") triggers a token to be minted in
**your backend** and verified inside **Sequent's Keycloak**, which then drops the
voter into the voting portal already authenticated.

### Why the link points at Keycloak

In this generation of the platform, identity lives in **Keycloak**, with one
realm per election event. The shared secret, the census (the realm's users) and
the login session all live there, so that is where the token is verified. The
voting portal is just an OIDC client; it never sees the secret.

## The Smart Link format

```
https://<keycloak-host>/realms/tenant-<tenant-id>-event-<event-id>/smart-link/login?auth-token=<auth-token>
```

For example, with Keycloak at `vote.university.com`, tenant `acme` and election
event `150017`, a Smart Link for the voter whose `user-id` is
`example@sequentech.io` looks like:

```
https://vote.university.com/realms/tenant-acme-event-150017/smart-link/login?auth-token=
khmac:///sha-256;89034fa3af76759f6edc658260afb30106c243fe86b60b652f714
73fecbb8c4e/example@sequentech.io:AuthEvent:150017:vote:1780869273
```

**Only three things change between your first generation integration and this
one: the host (now Keycloak), the election event id (now folded into the realm
name), and the secret.** The `auth-token` itself — its envelope, message and
HMAC — is byte-for-byte identical, so existing token generators keep working.

:::note URI encoding
The `auth-token` is a URL query parameter and must be URI-encoded. In a browser
use
[`encodeURIComponent()`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/encodeURIComponent);
for manual testing, [urlencoder.org](https://www.urlencoder.org/) works. We
**strongly recommend always** encoding it, even if your current ids look safe.
:::

## The `auth-token` format

```
khmac:///sha-256;<code>/<message>
```

The `<code>` is the lowercase-hex HMAC-SHA256 of `<message>` keyed with the
shared secret, per
[FIPS 198-1](https://csrc.nist.gov/publications/detail/fips/198/1/final).

### The message format

```
<user-id: String>:AuthEvent:<election-event-id>:vote:<timestamp: Int>
```

For example:

```
example@sequentech.io:AuthEvent:150017:vote:1780869273
```

- **user-id** — a unique identifier of the voter, matched against the census.
  It must be **stable** for a given voter and election (so re-votes map to the
  same person), must **not contain a colon** (`:`), and must exist in the
  census. To avoid leaking real identifiers, it is good practice to use a
  salted SHA-256 hash of your internal id, with a per-election salt.
- **AuthEvent** / **vote** — fixed literals identifying the permission being
  granted. They must appear exactly as shown.
- **election-event-id** — binds the token to one election event. It must equal
  the `event-id` part of the realm in the URL, so a token minted for one event
  cannot be replayed against another.
- **timestamp** — the Unix time (seconds) at which the token was minted. It is
  what lets Sequent expire the token and reject future-dated ones, so the clocks
  of your backend and Sequent must be reasonably in sync.

### Generating the token

In any language with an HMAC library:

```python
import hmac, hashlib, time, urllib.parse

def build_smart_link(keycloak_host, tenant, event_id, user_id, secret):
    message = f"{user_id}:AuthEvent:{event_id}:vote:{int(time.time())}"
    code = hmac.new(secret.encode(), message.encode(), hashlib.sha256).hexdigest()
    token = f"khmac:///sha-256;{code}/{message}"
    realm = f"tenant-{tenant}-event-{event_id}"
    return (f"https://{keycloak_host}/realms/{realm}/smart-link/login"
            f"?auth-token={urllib.parse.quote(token, safe='')}")
```

Or, equivalently, with the shell:

```bash
M="example@sequentech.io:AuthEvent:150017:vote:$(date +%s)"
CODE=$(printf '%s' "$M" | openssl dgst -sha256 -hmac "the cake is in the oven" -r | cut -d' ' -f1)
echo "khmac:///sha-256;$CODE/$M"
```

## Configuration in Sequent

### The shared secret

The shared secret is stored as a **realm attribute** on the election event's
realm. It is set per election event, so each event can have its own secret. An
election manager configures it via the election event settings (which call the
`update-realm-attributes` admin endpoint); the attribute keys are:

| Attribute | Meaning | Default |
| --- | --- | --- |
| `smart-link-shared-secret` | The HMAC key. **Required** to enable the feature. | _(unset → disabled)_ |
| `smart-link-timeout-secs` | How long a token stays valid after its timestamp. | `90` |
| `smart-link-clock-skew-secs` | Tolerance for tokens slightly ahead of Sequent's clock. | `5` |
| `smart-link-client-id` | OIDC client the voter lands in. | `voting-portal` |
| `smart-link-force-create` | Create voters not in the census (leave `false`). | `false` |

If `smart-link-shared-secret` is unset, the endpoint rejects every token, so the
feature is off until you configure a secret.

### The census

Because Sequent does authorization, **the voter's `user-id` must be in the census**
(the realm's users) before they follow the link. Upload the census as usual. With
`smart-link-force-create` left at its default `false`, a Smart Link for an unknown
`user-id` simply fails authentication.

## Security considerations

- **Mint in the backend, never in the browser.** The shared secret must never
  reach the voter's device, so the token must be generated server-side over an
  authenticated, TLS-protected request.
- **Tokens are bearer tokens.** Anyone who obtains the link can use it until it
  expires, so keep links out of logs, analytics and referrers, and keep the
  validity window short (the 90 s default is recommended).
- **Mint at click time.** Do not pre-generate links; generate one when the voter
  acts, so it is consumed well within its validity window.
- **Keep clocks in sync.** Sequent rejects both expired tokens and tokens minted
  in the future (beyond `smart-link-clock-skew-secs`), so use NTP on both sides.
- **Errors are deliberately vague.** Every failure returns the same generic
  message, to avoid revealing which check failed. Diagnose using the Keycloak
  server logs, which record the specific reason.

## Testing manually

You can exercise the endpoint without your application:

1. Configure `smart-link-shared-secret` on the event realm and add a test voter
   to the census.
2. Build a token with the snippet above (using the current `date +%s`).
3. Open the resulting URL in a browser — you should be redirected into the
   voting portal, logged in and ready to cast a vote.

If it fails, check the Keycloak logs for a line like
`SmartLink HMAC rejected: error=TOKEN_EXPIRED ...`, which names the exact reason.
