---
id: admin_portal_tutorials_voter_pin_link
title: Letting Voters Obtain Their PIN Through a One-Time Link
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Overview

Voters can obtain a PIN from a personal link in their credentials email, without an administrator
sending the PIN itself:

1. The voter opens the link, which is short enough to type.
2. The voter enters their username, for example a membership number, and selects
   **Obtain Your PIN**.
3. The page shows the PIN once, to write down or print. The voter logs in with it.
4. The link stops working.

Knowing a username and an email address is not enough: without the link there is nothing to submit
them to. The page uses the realm's login theme and layout, and asks only for the username.

The link is off by default and is configured per election-event realm.

## Turning It On

### 1. Declare the PIN as a secret attribute

In the event realm's **User profile**, declare the attribute that holds the PIN, by default `pin`,
as described in
[Protecting a Voter Attribute as Secret](./99-admin_portal_tutorials_add-user-attributes-to-keycloak.md#protecting-a-voter-attribute-as-secret):
annotation `sequent.secret=true`, view and edit permissions for administrators only.

The link stays off if the attribute is not a secret, if voters can view it, if the realm declares
the generated attributes below, or if the realm's unmanaged attributes policy is `ENABLED`. Keycloak
then logs a warning and the link answers `404`. Importing into such a realm or sending the link
fails with an error when the PIN is not a declared secret.

### 2. Set the realm attributes

```bash
kcadm.sh update realms/<event-realm> \
  -s 'attributes."pin-link-policy"=ONE_TIME_LINK' \
  -s 'attributes."pin-link-expires-at"=2026-12-31T23:59:59Z'
```

| Realm attribute | Default | Meaning |
|---|---|---|
| `pin-link-policy` | `DISABLED` | `ONE_TIME_LINK` turns the link on. Any other value leaves it off. |
| `pin-link-expires-at` | none, required | RFC 3339 instant, with a zone, when every link stops working. Without a valid value the link stays off. |
| `pin-link-max-attempts` | `5` | Wrong usernames that lock one link, from 1 to 100. |
| `pin-link-pin-attribute` | `pin` | The secret attribute with the PIN. |
| `pin-link-token-hash-attribute` | `pin-link-token-hash` | Generated attribute with the link's lookup hash. Do not declare it in the User Profile. |
| `pin-link-attempts-attribute` | `pin-link-attempts` | Generated attribute with the wrong-username count. Do not declare it in the User Profile. |
| `pin-link-public-base-url` | none | Short host for the links, see [Short links](#short-links). |

An invalid value turns the link off rather than falling back to a default. Harvest and Windmill read
realm attributes through a cache of up to 30 seconds, so wait that long after a change before
importing or sending.

Turn the link on **before** importing voters: see
[Voters imported before the link was on](#voters-imported-before-the-link-was-on).

### 3. Import the voters

Add a column named after the PIN attribute to the
[voter CSV](./08-admin_portal_tutorials_import-voters.md). It is encrypted like any secret attribute.

```csv
username,email,first_name,last_name,pin
10000001,ada@example.org,Ada,Example,482910375516
```

- The PIN is also the password, because a voter logs in with the PIN the link shows. A file without
  a `password` or `hashed_password` column uses the PIN as the password, and then every row needs one.
- A row whose `password` differs from its PIN is refused, and so is a row with several PIN values.
  A `hashed_password` cannot be compared: making it match the PIN is up to whoever made the file.
- A file with a `password` column may leave the PIN of a row blank. That voter has no link.
- The link's token is never imported. Columns named `pin_link_token`, `pin_link_url` or after a
  generated attribute are refused.
- Import only inserts new voters, so importing cannot give a used link a second life.

### 4. Send the link

In a [voter communication](./15-admin_portal_tutorials_voter-communication.md) use the two variables:

```handlebars
{{user.pin_link_url}}
{{user.pin_link_token}}
```

`pin_link_url` is the whole link and `pin_link_token` is the token alone, in groups of four,
for example `26C6-YCRD-HTVP-RZMB-K1BK-BEAJ-CV`. They are handled like secret voter fields: the Admin
Portal declares them when they appear in the content, sending requires
`voter-secret-attribute-read`, and the use is recorded in the electoral log. The PIN itself is not
needed in the message and should stay out of it. Sending fails with an error if they are declared
while the link is off, rather than mailing voters an empty link. Through the API, a template that
uses them without declaring them renders them as empty.

A voter without a usable PIN is skipped with a warning and counted as failed. The rest still get
their message.

Sending again gives the same voter the same link. It does not give a used or locked link another
life: the message would contain a dead link.

## What the Voter Sees

Whatever goes wrong, the voter sees the same message: an unknown, used, locked or expired link, a
disabled voter and a wrong username cannot be told apart from the page. The operators can tell them
apart from the logs, see [Auditing](#auditing).

Text comes from the message keys below, in the language the login pages would use. A realm can
override any of them under **Realm settings** > **Localization**.

| Key | Where |
|---|---|
| `pinLinkTitle`, `pinLinkIntro`, `pinLinkUsername`, `pinLinkSubmit` | The form |
| `pinLinkGenericError` | The refusal |
| `pinLinkRevealTitle`, `pinLinkRevealIntro`, `pinLinkRevealWarning`, `pinLinkPrint` | The PIN |

Printing uses the browser's print dialog. The PIN is in that one response only, so reloading the page
does not show it again.

## Short Links

Without `pin-link-public-base-url` the link is
`<KEYCLOAK_PUBLIC_URL>/realms/<event-realm>/pin-link/<token>`, and a realm name alone is about 86
characters. To have a link that can be typed, set it to a short host, for example
`https://pin.example.org`, so that the link is `https://pin.example.org/<token>`.

This needs one ingress rule per election event, since the token alone does not say which realm it
is for. For nginx, with `<event-realm>` and the upstream filled in:

```nginx
server {
    listen 443 ssl;
    server_name pin.example.org;

    # The token is in the path. Do not log requests here, or log them without the path.
    access_log off;

    location ~ "^/(?<token>[0-9A-Za-z-]{1,40})$" {
        proxy_pass http://keycloak/realms/<event-realm>/pin-link/$token$is_args$args;
        proxy_set_header Host <keycloak-public-host>;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # The login theme's stylesheets, fonts and images.
    location /resources/ {
        proxy_pass http://keycloak/resources/;
        proxy_set_header Host <keycloak-public-host>;
    }

    location / {
        return 404;
    }
}
```

The direct Keycloak path keeps working behind this rule, and its requests also carry the token in
the path. Exclude `/realms/*/pin-link/` from the access logs of every proxy, load balancer and web
application firewall in front of Keycloak. A token in a log is not enough to obtain a PIN without
the username, but it is a credential and should be treated as one.

## Changing a PIN or a Password

- **Setting a new PIN** in the voter editor, or through `/edit-user`, kills the old link and arms a
  new one. Send the communication again to deliver it. The new PIN becomes the password too, which
  requires `voter-change-password` as well as `voter-secret-attribute-write`. A request bringing a
  different password is refused.
- **Clearing the PIN** removes the link and leaves the password alone.
- **Setting a new password alone**, in the voter editor or through a voter information letter, removes
  the link, because it would show a PIN that no longer logs in.
- **A voter changing their own password** is not visible to the link. If the voter had not used the
  link yet, it still shows the original PIN, which no longer logs in.
- **Creating a voter** with a PIN in the voter editor arms the link and sets the password the same
  way as a new PIN.

### Voters imported before the link was on

Voters imported while the policy was off have a PIN but no link, and a message would render a link
that does not work. Importing cannot fix them, because import only inserts. Set the PIN again on
those voters to arm their links.

## Security Properties

- **A `GET` never consumes the link**, so mail scanners prefetching it do no harm. Only a `POST` with
  the right username does.
- **One winner.** The link is spent by deleting its hash in one database statement, committed before
  the PIN is shown. Concurrent requests, other replicas and restarts cannot reveal it twice.
- **Wrong usernames lock the link** after `pin-link-max-attempts`. A locked link does not unlock by
  itself; setting the PIN again arms a new one. They also count in Keycloak's brute-force detection
  when the realm has it on, in the same counter as failed logins, and a voter temporarily locked out
  there is refused too. That includes the quick-login check: two wrong usernames within the realm's
  quick-login window, a second by default, suspend the voter for a while, and attempts made during
  the suspension neither count towards the lock nor reveal the PIN. Without a link nothing is
  submitted, so usernames cannot be tried this way.
- **The lookup is by the token**, not by the username, so a stolen link cannot be used to try every
  username for a match.
- **The token cannot reveal the PIN.** It is an HMAC-SHA-256 of the PIN under a key derived from the
  master secret, scoped to the tenant, event and voter, truncated to 130 bits, and written in
  case-insensitive Crockford base32. A plain hash would be enough to brute-force a 12-digit PIN from
  the link.
- **Only a hash of the token is stored**, in a generated attribute that the account console, tokens
  and the Admin Portal do not see. The generated attributes cannot be written through Harvest or
  imported, and they are left out of exports and templates.
- **The PIN is in the `POST` response only**, with `Cache-Control: no-store`,
  `Referrer-Policy: no-referrer` and a strict `Content-Security-Policy`. The page loads nothing from
  other origins.

## Auditing

Keycloak logs and raises a `CUSTOM_REQUIRED_ACTION` event, with detail `pin_link_stage`, each time a
link is opened (`opened`), refused (`refused`, error `pin_link_denied`, with the reason) or revealed
(`revealed`), for the voter the link belongs to. The events reach the electoral log as Keycloak user
events that read `pin_link opened`, `pin_link refused <reason>` and `pin_link revealed`, so every
`GET` of a valid link adds an entry, including from a mail scanner. A `GET` of an unknown token
records nothing. Neither carries the PIN or the token.

Each reveal is also recorded in the election event's electoral log as
`VOTER_SECRET_ATTRIBUTE: PIN_LINK_REVEAL`, naming the voter and the attribute. The broker must
confirm this entry before the link is spent: when it cannot be recorded, the voter is refused and
the link stays alive.

See also the [Secret Voter Attribute Boundary](../../05-reference/03-software-architecture/backend-components.md#secret-voter-attribute-boundary).
