---
id: election_management_election_event_data
title: Data
---

## Keycloak realm secrets in exports and imports

Keycloak exports `**********` in place of most secrets of the event's Keycloak realm, so an
election-event export does not contain them. When the export is imported, settings that hold
`**********` are left unset instead of being saved as `**********`, and the import task's log lists
each one in a `Warning:` line that names the setting, never its value. Re-enter them in the
Keycloak admin console of the imported event's realm:

| Setting | Where to re-enter it |
|---------|----------------------|
| SMTP server password | **Realm settings > Email** |
| Identity provider client secret | **Identity providers >** the provider |
| Component secrets, such as an LDAP bind credential | **User federation >** the provider |

Authenticator settings, such as an integration's API key, are exported and imported as they are.

Client secrets are not listed: the import sets the platform client's secret from the deployment
and Keycloak generates new ones for the other clients. It also drops the clients' rotated secrets
and registration access tokens. Vault references (`${vault.…}`) are not secrets in the export and
are imported as they are.
