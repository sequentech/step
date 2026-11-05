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

Keycloak needs the same master secret as Harvest and Windmill, in its `MASTER_SECRET` environment
variable, as described in the
[Secret Voter Attribute Boundary](../../05-reference/03-software-architecture/backend-components.md#secret-voter-attribute-boundary).
The supplied Compose files pass it. A deployment that configures Keycloak on its own has to add it.
Without it no PIN can be revealed: voters get the generic error and the link is kept, and Keycloak
logs an error naming the variable. Keycloak also needs `AMQP_ADDR` and `ENV_SLUG`, the same it uses
for electoral log events, to record each reveal.

### 1. Declare the PIN as a secret attribute

In the event realm's **User profile**, declare the attribute that holds the PIN, by default `pin`,
as described in
[Protecting a Voter Attribute as Secret](./99-admin_portal_tutorials_add-user-attributes-to-keycloak.md#protecting-a-voter-attribute-as-secret):
annotation `sequent.secret=true`, view and edit permissions for administrators only.

The link stays off if the attribute is not a secret, if voters can view or edit it, if the realm
declares the generated attributes below, or if the realm's unmanaged attributes policy is `ENABLED`. Keycloak
then logs a warning and the link answers `404`. Importing into such a realm or sending the link
fails with an error, so no message carries a dead link.

Leave the unmanaged attributes policy unset. Windmill cannot read a realm that sets it to any value,
so with `ADMIN_VIEW` or `ADMIN_EDIT` too the link answers, but every import and send fails with an
`unknown variant` error.

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
| `pin-link-asset-origins` | none | Other hosts the page may load images and fonts from, see [Style](#style). |

An invalid value turns the link off rather than falling back to a default, with two exceptions. A
bad entry in `pin-link-asset-origins` is left out with a warning in Keycloak's log.
`pin-link-public-base-url` is used as written, so a wrong value mails links that do not work: send
one to yourself first. Harvest and Windmill read realm attributes through a cache of up to 30
seconds, so wait that long after a change before importing or sending.

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

Printing uses the browser's print dialog. The PIN is in that one response only, so reloading the page
does not show it again.

### Text and Languages

Text comes from the message keys below, in the language the login pages would use: the browser's
language among the realm's supported locales, or else the realm's default. A realm that forces its
default language shows that one unless the voter picks another in the menu. A realm can override any
of them, per language, under **Realm settings** > **Localization**.

| Key | Where |
|---|---|
| `pinLinkTitle`, `pinLinkIntro`, `pinLinkUsername`, `pinLinkSubmit` | The form |
| `pinLinkGenericError` | The refusal |
| `pinLinkRevealTitle`, `pinLinkRevealIntro`, `pinLinkRevealWarning`, `pinLinkPrint` | The PIN |

These keys ship in English, Spanish, Catalan, Basque, Galician and Tagalog. An override applies to the
language it is set for: a realm that overrides `pinLinkIntro` in English and also offers Spanish still
shows the Spanish text that ships, until it is overridden in Spanish too.

When the realm offers more than one language, the form has the theme's language menu. The page that
shows the PIN does not, because leaving it would lose the PIN.

### Style

The page uses the realm's login theme, and its `loginCustomCss` message as the login pages do. These
selectors are stable:

| Selector | What |
|---|---|
| `#pin-link-form` | The form |
| `#username` | The username field |
| `#pin-link-form [type=submit]` | The button that obtains the PIN |
| `#pin-link-value output` | The PIN |
| `#pin-link-print` | The print button |

For example, under **Realm settings** > **Localization**, in the realm's default language:

```text
loginCustomCss = #pin-link-value output '{' color: #0f054c; font-size: 2.5rem; '}'
```

- `loginCustomCss` is a message, so braces are written `'{'` and `'}'` and a single quote is doubled.
- It wins over the page's own rules without needing `!important`, but the theme's stylesheets load
  after it, so overriding a theme rule takes a more specific selector or `!important`.
- A language with no `loginCustomCss` of its own uses the one in the realm's default language.

The page's Content-Security-Policy allows no other hosts by default, so an image or font that
`loginCustomCss` loads from another host does not show here. To show a logo, either serve it from
the Keycloak theme, embed it as a `data:` image, or list its host in the realm attribute
`pin-link-asset-origins`:

```bash
kcadm.sh update realms/<event-realm> \
  -s 'attributes."pin-link-asset-origins"=https://assets.example.org'
```

- List `https://` origins, separated by spaces or commas, with no path and no wildcard. A bad entry
  is left out with a warning in Keycloak's log, and the link keeps working.
- If the image redirects to another host, list that host too: the policy applies to each step.
- They apply to images and fonts only. A stylesheet or a script from another host cannot be allowed,
  because either could read the PIN on the page. Put the realm's CSS in `loginCustomCss` instead.
- The hosts listed see the voter's IP address when the page loads, including on the page with the
  PIN. They do not see the link: for an image that CSS loads, Chromium sends Keycloak's origin as
  the referrer despite the page's `no-referrer`, but never the path with the token. An image or a
  font cannot read the page.

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

If Keycloak is served under a relative path such as `/auth`, add it to both `proxy_pass` targets.

The direct Keycloak path keeps working behind this rule, and its requests also carry the token in
the path, so the access logs of the proxies, load balancers and web application firewalls in front
of Keycloak can hold it. A token in a log is not enough to obtain a PIN without the username, and it
stops working once the voter has used it. Until then, anyone who can read those logs and knows the
username could use the link, so limit who can read them.

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
- **An edit that is refused** leaves the link alone, whether Keycloak refuses it or, in an election
  event linked to an external voter system, the task that applies the edit.
- **A task that fails after Keycloak accepted the edit** leaves the edit in place. It says so in its
  error: when the error says the link was not updated, repeat the edit. A failed entry in the
  electoral log does not stop the link from changing or a disabled voter's ballots from being
  discarded.

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
  other origins, except the images and fonts a realm allows in `pin-link-asset-origins`. No attribute
  allows a script or a stylesheet from another host, because either could read the PIN.

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
the link stays alive. With a broker that has gone away or stopped answering, that refusal can take
up to about 15 seconds, and reveals on the same Keycloak replica wait their turn.

See also the [Secret Voter Attribute Boundary](../../05-reference/03-software-architecture/backend-components.md#secret-voter-attribute-boundary).
