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

The **Attribute Transform and Lookup** identity provider mapper handles both. It works with SAML
and OIDC identity providers and runs these steps on every login:

1. **Source:** reads one or more SAML attributes or OIDC claims.
2. **Extract:** keeps part of each value, using a regular expression (optional).
3. **Normalize:** trims each value and sets its case.
4. **Join:** joins the values with a separator when there are several sources.
5. **Validate:** checks the joined value against a regular expression (optional).
6. **Lookup:** translates the value through a table (optional).
7. **Target:** stores the result in a user attribute, or in the username of a new user.

If a step produces no value, the mapper leaves the target unset, stores a default value, or
rejects the login, as configured.

---

## Prerequisites

- Access to the Keycloak Admin Console.
- The `sequent.idp-attribute-mapper.jar` extension in Keycloak's `providers/` directory (included
  in the Sequent Keycloak Docker image).
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
| **Validation regex** | The joined value must match it entirely, before the lookup. Otherwise it is a miss. | none |
| **Lookup table** | Translates the joined value (see [Lookup table format](#lookup-table-format)). A value with no row is a miss. Without a table, the joined value is stored. | none |
| **Target** | `USER_ATTRIBUTE` stores the value in **Target attribute**. `USERNAME` sets the username of users created on first login; existing usernames never change. | `USER_ATTRIBUTE` |
| **Target attribute** | User attribute to store the value in. `email`, `firstName` and `lastName` set those user properties. | — |
| **On miss** | What to do when a source is missing, extraction or validation fails, or the table has no row (see [Behavior summary](#behavior-summary)). | `UNSET` |
| **Default value** | Value stored on a miss when **On miss** is `DEFAULT`. | — |
| **Error message** | Message key or text shown to the voter when **On miss** is `REJECT`. | Generic IdP error |

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

The table is parsed once and kept in memory until the mapper configuration changes. A login only
does an in-memory lookup and makes no network calls.

---

## Behavior Summary

| On miss | New user | Returning user (`force` sync mode) |
|---|---|---|
| `UNSET` | The target stays empty. | The attribute is removed. |
| `DEFAULT` | The default value is stored. | The default value is stored. |
| `REJECT` | The login stops and the voter sees the error message. No user is created. | The login stops and the voter sees the error message. |

The mapper checks the values on every login, whatever the sync mode, so `REJECT` also applies to
returning voters.

---

## Example – SAML: Voter ID from the NameID

The IdP sends a NameID such as `3f2b8c1a-9d4e-8a7b-b1c2-0d9e8f7a6b5c@idp.example.org`. The
username must be the part before `@`, and it must be a UUID version 8.

| Parameter | Value |
|---|---|
| Sync mode override | `import` |
| Sources | `$nameid` |
| Extract regex | `^([^@]+)@` |
| Case | `LOWER` |
| Validation regex | `^[0-9a-f]{8}-[0-9a-f]{4}-8[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$` |
| Target | `USERNAME` |
| On miss | `REJECT` |
| Error message | `Your voter ID is not valid. Contact your identity provider.` |

## Example – SAML: Area from Region and Precinct

The IdP sends the attributes `region` (`09`) and `precinct` (`0150`).

| Parameter | Value |
|---|---|
| Sync mode override | `force` |
| Sources | `region`, `precinct` |
| Separator | `:` |
| Lookup table | The table from [Lookup table format](#lookup-table-format) |
| Target | `USER_ATTRIBUTE` |
| Target attribute | `area-id` |
| On miss | `REJECT` |
| Error message | `Your precinct is not part of this election.` |

With `force`, a voter whose precinct changes at the IdP gets the new area on the next login.

## Example – OIDC

The same mappers work with an OpenID Connect identity provider. Use claim names as **Sources**,
for example `region` and `precinct`, or a dotted path such as `address.region`. The mapper looks
for each claim in the access token, then the ID token, then the user info response. Request the
scopes that release those claims in the identity provider's **Scopes** setting.

---

## Loading a Large Table through the Admin REST API

A table for a large election can have tens of thousands of lines, which is impractical to paste
into the Admin Console. Load it with the Admin REST API instead.

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
the identity provider.
