---
id: x509_cert_login_events
title: X.509 Certificate Login — Electoral Log Events
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# X.509 Certificate Login — Electoral Log Events

## Overview

When a voter authenticates using an X.509 certificate, Keycloak fires a `LOGIN`
or `LOGIN_ERROR` event that `CustomEventListenerProvider` publishes to RabbitMQ
and ultimately stores in immudb via `enqueue_electoral_log_event`. This document
describes what information is captured in those events and why.

**See also:** [X.509 Client Certificate Authentication — Architecture](x509_client_cert_architecture)

---

## Authentication Flow

The relevant steps in the mTLS login flow are:

1. **`X509CertClassifierAuthenticator`** — reads the client certificate from the
   reverse-proxy header, extracts the issuer CN, and records two event details:
   `voter_cert_subject_dn` and `ca_cert_issuer_cn`. Runs before any other X.509
   step.
2. **`ConditionalAuthNoteAuthenticator`** — routes to the correct X.509 sub-flow
   based on `cert-type`.
3. **Keycloak built-in `X509ClientCertificateAuthenticator`** — validates the
   certificate chain against the per-realm CA bundle provided by
   `UrlTruststoreProvider`.
4. **`CustomEventListenerProvider`** — receives the resulting `LOGIN` or
   `LOGIN_ERROR` event and publishes it to RabbitMQ.
5. **`enqueue_electoral_log_event`** (Windmill) — consumes the queue and stores
   the event in immudb.

---

## Certificate Detail Fields

Both fields are set by `X509CertClassifierAuthenticator` in every branch of its
`authenticate()` method, so they are always present on any event that reaches the
electoral log when the authentication attempt used the mTLS flow.

| Field | Content | Example |
|---|---|---|
| `voter_cert_subject_dn` | Subject DN of the voter's certificate (RFC 2253 format) | `CN=GARCIA LOPEZ JUAN,C=ES,...` |
| `ca_cert_issuer_cn` | CN of the certificate's issuer (the matched CA) | `AC FNMT Usuarios` |

When no certificate is present in the header, or when the certificate cannot be
parsed, both fields are set to `"none"`.

---

## Event Body Format

`CustomEventListenerProvider` constructs the `body` field of the RabbitMQ message
as follows for `LOGIN`/`LOGIN_ERROR` events where both cert fields are present:

```
LOGIN:        voter_cert_subject_dn=<dn> ca_cert_issuer_cn=<cn>
LOGIN_ERROR:  <error_code> voter_cert_subject_dn=<dn> ca_cert_issuer_cn=<cn>
```

`<error_code>` is the value of `event.getError()`, which Keycloak populates from
[`org.keycloak.events.Errors`](https://github.com/keycloak/keycloak/blob/26.4.0/server-spi-private/src/main/java/org/keycloak/events/Errors.java).

Events that do not have both cert detail fields fall through to the default
behaviour: `body = event.getError()`.

---

## Observed Event Patterns

### Successful login

```
LOGIN: voter_cert_subject_dn=CN=Mario Pérez Sánchez - 12345678A,2.5.4.4=...,C=ES ca_cert_issuer_cn=AC FNMT Usuarios
```

### Voter not registered in this election event

Two LOGIN_ERROR events fire in sequence:

```
LOGIN_ERROR: user_not_found voter_cert_subject_dn=C=US,O=Sequent Test,CN=voter@sequent.test ca_cert_issuer_cn=Sequent Dev CA
LOGIN_ERROR: access_denied voter_cert_subject_dn=C=US,O=Sequent Test,CN=voter@sequent.test ca_cert_issuer_cn=Sequent Dev CA
```

The first event carries `user_not_found` (the actual cause). The second carries
the generic `access_denied` that Keycloak fires as part of the login failure
sequence.

### Wrong password (certificate-less flow or mixed flow)

```
LOGIN_ERROR: invalid_user_credentials
```

No cert fields — the attempt did not go through `X509CertClassifierAuthenticator`.

### Certificate chain not trusted

When Keycloak's built-in `X509ClientCertificateAuthenticator` cannot build the
certificate path against the CA bundle — for example because the CA was not
added to the election event's truststore, or because the Cloudflare mTLS
truststore is out of sync — only `access_denied` appears:

```
LOGIN_ERROR: access_denied voter_cert_subject_dn=CN=fake-voter@sequent.test ca_cert_issuer_cn=Sequent Dev CA
```

#### Why the specific crypto error is not captured

Keycloak logs the underlying `CertPathBuilderException` internally (e.g.
`"No issuer certificate for certificate in certification path found."`) but
catches it inside `X509ClientCertificateAuthenticator.authenticate()` before the
event fires. By the time `CustomEventListenerProvider.onEvent()` is invoked, only
`error = "access_denied"` is available — the exception message is not propagated
to the Keycloak event system. The full stack trace is visible in the Keycloak pod
logs alongside the `LOGIN_ERROR` event timestamp.

To diagnose a chain-of-trust failure, check:
1. That the CA certificate is imported in the election event's "Certificate
   Authorities" tab in the admin portal.
2. In production, that the same CA is also present in the Cloudflare mTLS
   truststore (requires a manual update — see the architecture doc).
3. The Keycloak pod logs for the `CertPathBuilderException` that is logged at
   ERROR level immediately before the `LOGIN_ERROR` event.

---

## Data Flow

```
X509CertClassifierAuthenticator.authenticate()
  context.getEvent().detail("voter_cert_subject_dn", subjectDn)
  context.getEvent().detail("ca_cert_issuer_cn", issuerCn)
        │
        ▼
Keycloak fires LOGIN / LOGIN_ERROR
        │
        ▼
CustomEventListenerProvider.onEvent(Event event)
  body = "voter_cert_subject_dn=... ca_cert_issuer_cn=..."            // LOGIN
  body = error + " voter_cert_subject_dn=... ca_cert_issuer_cn=..."   // LOGIN_ERROR
  → publish to RabbitMQ { body, user_id, username, ... }
        │
        ▼
enqueue_electoral_log_event(LogEventInput { body: Plain(body), ... })
        │
        ▼
build_keycloak_event_message(..., error_message = body, ...)
        │
        ▼
stored in immudb (ElectoralLogMessage)
```

---

## Relevant Source Files

| File | Role |
|---|---|
| `packages/keycloak-extensions/conditional-authenticators/…/X509CertClassifierAuthenticator.java` | Sets `voter_cert_subject_dn` and `ca_cert_issuer_cn` event details |
| `packages/keycloak-extensions/custom-event-listener/…/CustomEventListenerProvider.java` | Constructs the event body and publishes to RabbitMQ |
| `packages/keycloak-extensions/message-otp-authenticator/…/Utils.java` | Defines the shared `VOTER_CERT_SUBJECT_DN` and `CA_CERT_ISSUER_CN` constants |
| `packages/keycloak-extensions/url-truststore-provider/…/UrlTruststoreProvider.java` | Supplies the per-realm CA bundle used during cert chain validation |
