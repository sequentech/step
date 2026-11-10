---
id: admin_portal_tutorials_idp_attribute_transform_lookup
title: Transforming and Looking Up IdP Attributes
---

<!--
-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Overview

When voters log in through an external Identity Provider (IdP), the IdP often sends values that
Step can't store as they are:

- The value needs a **transform**. For example, the voter ID arrives as `<id>@<domain>` and the
  username must be `<id>`.
- The value has to be **translated** into a Step value. For example, the IdP sends a region and a
  precinct, and Step needs the voter's area.

Two pieces handle this, each with one job:

- The **Attribute Transform and Lookup** identity provider mapper computes the value. It works
  with SAML and OIDC identity providers and runs these steps on every login:

  1. **Source:** reads one or more SAML attributes or OIDC claims.
  2. **Extract:** keeps part of each value, using a regular expression (optional).
  3. **Normalize:** trims each value and sets its case.
  4. **Join:** joins the values with a separator when there are several sources.
  5. **Lookup:** translates the value through a table (optional).
  6. **Target:** stores the result in a user attribute, or in the username of a new user.

  If a step produces no value, the mapper leaves the target unset or stores a default value.

- The **Condition - Brokered Value** condition checks the result in the identity provider's login
  flows. Together with Keycloak's **Deny access** step, it rejects logins whose values are missing
  or invalid, before the user is created (see [Rejecting logins](#rejecting-logins)).

---

## Prerequisites

- Access to the Keycloak Admin Console.
- The `sequent.idp-attribute-mapper.jar` and `sequent.conditional-authenticators.jar` extensions
  in Keycloak's `providers/` directory (included in the Sequent Keycloak Docker image).
- A SAML or OpenID Connect identity provider configured in the election event realm.

---

## Step 1 – Add the Mapper

1. In the Keycloak Admin Console, select the election event realm.
2. Navigate to **Identity providers** and select the identity provider.
3. Open the **Mappers** tab and click **Add mapper**.
4. Set **Mapper type** to **Attribute Transform and Lookup**.
5. Fill in the configuration (see below) and click **Save**.

Add one mapper for each value you need, for example one for the username and one for the area.

---

## Configuration

| Parameter | Description | Default |
|---|---|---|
| **Sync mode override** | `import` sets the value only when the user is created. `force` updates it on every login. `legacy` behaves like `force`. | Identity provider's sync mode |
| **Sources** | SAML attribute names (or friendly names) or OIDC claims, in order. Use `$nameid` for the SAML NameID and `$id` for the brokered user ID (the SAML principal or the OIDC `sub`). The first value of each source is used. | — |
| **Extract regex** | Applied to each source value. The first capture group is kept, or the whole match if the expression has no group. A value that doesn't match is a miss. | none |
| **Separator** | Joins the values when there are several sources. | `:` |
| **Case** | `NONE`, `LOWER` or `UPPER`, applied to each value after trimming it. Lookup table keys get the same case. | `NONE` |
| **Lookup table** | Translates the joined value (see [Lookup table format](#lookup-table-format)). A value with no row is a miss. Without a table, the joined value is stored. | none |
| **Target** | `USER_ATTRIBUTE` stores the value in **Target attribute**. `USERNAME` sets the username of users created on first login; existing usernames never change. | `USER_ATTRIBUTE` |
| **Target attribute** | User attribute to store the value in. `email`, `firstName` and `lastName` set those user properties. | — |
| **On miss** | What to do when a source is missing, extraction fails, or the table has no row (see [Behavior summary](#behavior-summary)). | `UNSET` |
| **Default value** | Value stored on a miss when **On miss** is `DEFAULT`. | — |

:::note
Step reads the voter's area from the user attribute `area-id`. Use that name as the
**Target attribute** of an area mapper.
:::

---

## Lookup Table Format

Each line is one of:

- `<value> = <key>, <key>, <from>..<to>`: every listed key, and every key in each range, maps to
  `<value>`.
- `<key>,<value>`: one key per line, for tables generated from another system.

Empty lines and lines that start with `#` are ignored. Both formats can be mixed.

A **range** covers the keys that share the prefix of `<from>` and whose last run of digits lies
between the digits of `<from>` and `<to>`. `<to>` can repeat the prefix or give only the digits.
When both bounds have the same number of digits, a key must have that many digits too:
`09:0001..09:0420` matches `09:0007` but not `09:7`. An exact key wins over a range.

```text
# area id = region:precinct keys
4b1f0c1e-3a3e-4f6e-9d1a-0c6c8f1d2a11 = 09:0001..0420, 09:0500
7c2e9b44-8d0f-4b9a-a1e3-5f2d6b7c8e22 = 09:0421..0499, 09:0501..0900
```

The table is checked when it is first used. Duplicate keys with different values, overlapping
ranges, reversed ranges and lines in neither format make the configuration invalid. A login
through an identity provider with an invalid mapper fails with a generic error, and the Keycloak
log says which line is wrong.

The table is parsed once and kept in memory until the mapper configuration changes. A login looks
the value up in memory and makes no network calls. Keycloak still reads the mapper configuration,
table included, from its database on every login, so keep large tables compact (see
[Loading a large table](#loading-a-large-table-through-the-admin-rest-api)).

---

## Behavior Summary

| On miss | New user | Returning user (`force` sync mode) |
|---|---|---|
| `UNSET` | The target stays empty, even if the identity provider supplied a value for it. | The attribute or user property is removed. |
| `DEFAULT` | The default value is stored. | The default value is stored. |

`DEFAULT` can't be used with target `USERNAME`, because every user without a value would get the same
username and share one account. That configuration is invalid, so logins through the identity
provider fail with a generic error.

The mapper never stops a login. To reject a login on a miss, leave **On miss** as `UNSET` and add a
check to the login flows (see [Rejecting logins](#rejecting-logins)).

---

## Rejecting Logins

The mappers run before the identity provider's login flows, and they store their results in the
login's brokered identity. The **Condition - Brokered Value** condition reads those results, so a
flow can check them and stop the login with Keycloak's **Deny access** step.

| Parameter | Description | Default |
|---|---|---|
| **Target** | `USERNAME` checks the username. `USER_ATTRIBUTE` checks **Attribute**. | `USER_ATTRIBUTE` |
| **Attribute** | User attribute to check. `email`, `firstName` and `lastName` check those user properties. | — |
| **Regex** | The value must match it entirely. Without a regex, any non-empty value matches. | none |
| **Negate output** | Inverts the result. Set it to deny access when the value does **not** match. | off |

Add each check to two flows:

- **First broker login flow:** runs only on a voter's first login, before the user is created. A
  rejected voter gets no account.
- **Post login flow:** runs on every login through the identity provider, after the mappers have
  updated the user. It catches returning voters whose values stopped being valid, for example a
  precinct that left the election.

### Set up a check

1. In **Authentication**, duplicate the **first broker login** flow, for example as
   `voter first broker login`.
2. Click **Add sub-flow**, name it after the check (for example `invalid voter ID`), and move it to
   the top of the flow. Set its requirement to **Conditional**.
3. In the sub-flow, add the condition **Condition - Brokered Value** with requirement
   **Required**, and configure it (see the examples below).
4. In the sub-flow, add the step **Deny access** with requirement **Required**. In its settings, set
   **Error message** to the message key or text the voter should see.
5. Repeat steps 2–4 for each check.
6. Create a flow for the post login checks, for example `voter post login`, with the same sub-flows
   for the checks that apply to returning voters. End it with the step **Allow access** with
   requirement **Required**: a flow whose only steps are skipped conditions fails every login.
7. In **Identity providers**, select the identity provider and set **First login flow override**
   and **Post login flow** to the two flows.

Test every check with a login before voting opens.

---

## Example – SAML: Voter ID from the NameID

The IdP sends a NameID such as `3f2b8c1a-9d4e-8a7b-b1c2-0d9e8f7a6b5c@idp.example.org`. The
username must be the part before `@`, and it must be a UUID version 8.

Mapper:

| Parameter | Value |
|---|---|
| Sync mode override | `import` |
| Sources | `$nameid` |
| Extract regex | `^([^@]+)@` |
| Case | `LOWER` |
| Target | `USERNAME` |
| On miss | `UNSET` |

Check in the first broker login flow:

| Step | Setting | Value |
|---|---|---|
| Condition - Brokered Value | Target | `USERNAME` |
| | Regex | `^[0-9a-f]{8}-[0-9a-f]{4}-8[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$` |
| | Negate output | on |
| Deny access | Error message | `Your voter ID is not valid. Contact your identity provider.` |

If the NameID has no `@`, the mapper leaves the username as the identity provider sent it, and the
check rejects the login because a username the mapper couldn't resolve counts as empty. The
username of an existing user never changes, so this check only belongs in the first broker login
flow.

## Example – SAML: Area from Region and Precinct

The IdP sends the attributes `region` (`09`) and `precinct` (`0150`).

Mapper:

| Parameter | Value |
|---|---|
| Sync mode override | `force` |
| Sources | `region`, `precinct` |
| Separator | `:` |
| Lookup table | The table from [Lookup table format](#lookup-table-format) |
| Target | `USER_ATTRIBUTE` |
| Target attribute | `area-id` |
| On miss | `UNSET` |

Check in both the first broker login flow and the post login flow:

| Step | Setting | Value |
|---|---|---|
| Condition - Brokered Value | Target | `USER_ATTRIBUTE` |
| | Attribute | `area-id` |
| | Negate output | on |
| Deny access | Error message | `Your precinct is not part of this election.` |

With `force`, a voter whose precinct changes at the IdP gets the new area on the next login. If the
new precinct isn't in the table, the mapper removes `area-id` and the post login check rejects the
login.

## Example – OIDC

The same mappers and checks work with an OpenID Connect identity provider. Use claim names as **Sources**,
for example `region` and `precinct`, or a dotted path such as `address.region`. The mapper looks
for each claim in the access token, then the ID token, then the user info response. Request the
scopes that release those claims in the identity provider's **Scopes** setting.

---

## Loading a Large Table through the Admin REST API

A table for a large election can have tens of thousands of lines, which is impractical to paste
into the Admin Console. Load it with the Admin REST API instead.

Prefer the `<value> = <keys and ranges>` format for large tables. Consecutive keys with the same
value fit in one range, so the table stays small. For example, 70,000 precincts in 176 areas take
about 3 MB as one `<key>,<value>` line per precinct, and about 40 KB grouped into ranges. Because
Keycloak reads the whole configuration on every login, the one-line-per-key table made each
login about 30 ms slower in testing, and the grouped table about 10 ms.

1. Get an admin access token:

   ```bash
   TOKEN=$(curl -s -d grant_type=password -d client_id=admin-cli \
     -d username="$ADMIN_USER" -d password="$ADMIN_PASSWORD" \
     "$KEYCLOAK_URL/realms/master/protocol/openid-connect/token" | jq -r .access_token)
   ```

2. Find the mapper:

   ```bash
   curl -s -H "Authorization: Bearer $TOKEN" \
     "$KEYCLOAK_URL/admin/realms/$REALM/identity-provider/instances/$IDP_ALIAS/mappers" \
     | jq '.[] | select(.identityProviderMapper == "sequent-attribute-lookup-idp-mapper") | {id, name}'
   ```

3. Replace its table with the contents of `areas.txt`:

   ```bash
   MAPPER_ID=<id from step 2>
   curl -s -H "Authorization: Bearer $TOKEN" \
     "$KEYCLOAK_URL/admin/realms/$REALM/identity-provider/instances/$IDP_ALIAS/mappers/$MAPPER_ID" \
     | jq --rawfile table areas.txt '.config["lookup.table"] = $table' > mapper.json
   curl -s -X PUT -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
     --data @mapper.json \
     "$KEYCLOAK_URL/admin/realms/$REALM/identity-provider/instances/$IDP_ALIAS/mappers/$MAPPER_ID"
   ```

4. Check the Keycloak log on the next login. It reports the number of table entries and the parse
   time, or the line that makes the table invalid.

Test the new table with a login before voting opens: an invalid table fails every login through
the identity provider, and a table that misses precincts makes the area check reject those voters.
