---
id: developers_windmill
title: Developers Windmill
---

# Tally

## Discarded/Auditable Ballots

Discarded Ballots that for some reason are not included in the tally. Possible reasons:

- The voter was disabled.
- The voter was deleted.
- The ballot was cast outside the Voting Period.
- The voter is not authorized for the election of the ballot.
- The voter is not assigned to the area of the ballot.
- The ballot is from a previous revote (only the last vote counts). (Note, this is not counted as a discarded ballot yet).

## Eligible Voters

Eligible voters are voters that can vote. Voters not included here are disabled and deleted voters.
# Tenant realm clients

When Windmill creates a tenant realm from the template, it sets these clients:

| Client | Settings |
| --- | --- |
| `admin-portal` | Root URL and home URL `ADMIN_PORTAL_URL`. Valid redirect URIs `<ADMIN_PORTAL_URL>/*`. Web origins and valid post logout redirect URIs `+`. PKCE method `S256`. No valid request URIs. |
| `cli-account-admin` | No valid redirect URIs and no web origins. This client is for direct access grants only. |

Set `ADMIN_PORTAL_URL` in Windmill to the admin portal URL, for example
`https://admin.example.com`. Creating a tenant or importing a tenant
configuration fails while it is unset.

The same settings are in `.devcontainer/keycloak/import/`, which Keycloak
imports at startup. Deployments that keep their own copy of the tenant realm
template should apply the same settings to it.

Windmill does not change the clients of a realm that already exists. For each
existing `tenant-<id>` realm, apply the settings above to its `admin-portal` and
`cli-account-admin` clients in the Keycloak admin console, and list every URL
that serves the admin portal.
