---
id: election_management_election_event_certificates
title: Certificates
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Voters can authenticate using client certificates issued by a Certificate Authority (CA). For this to work, the relevant CAs must be uploaded to the election event. Different election events can have different sets of CAs configured.

---

## Enabling certificate authentication

Certificate authentication is disabled by default. To enable it:

1. Go to the election event's **Data** tab.
2. Open the **Advanced Configurations** section.
3. Enable the **Voter Digital Certificate Policy** setting.

Once enabled, a **Login with Certificate** button will appear on the voting portal login page, allowing voters to authenticate using their client certificate.

---

## Managing certificates

The **Certificates** tab lets you manage the CA certificates for the election event.

Certificates must be in **PEM format**. Importing a PEM file that contains multiple certificates will import all of them at once.

Available actions:

- **Import**: Upload a PEM file containing one or more CA certificates.
- **Export**: Download all certificates as a PEM file or select one or more and click on Export.
- **Delete**: Remove one or the selected certificates.
- **View**: Display the certificate's details in human readable format.

---

## Keycloak configuration

Uploading CA certificates to the admin portal is only part of the setup. Keycloak must also be configured separately to classify and authenticate voters based on those CAs.

### Certificate browser flow

Authentication is handled by the **certificate browser flow** in Keycloak. This flow contains two top-level subflows:

- **Credentials subflow** — classifies the certificate and resolves the voter identity.
- **Deny subflow** — handles error cases (see [Error cases](#error-cases) below).

### Classifier subflows

Inside **Credentials subflow**, the `x509-cert-classifier` authenticator runs first and sets a `cert-type` auth note based on the certificate's CN. Each CA is then handled by a **conditional subflow** that checks `cert-type` and, if it matches, runs the `x509-user-resolution` authenticator to look up the voter.

Two example subflows are created by default at election event creation and can be edited, removed, or used as a reference:

- **Dev Sequent** — matches certificates with `cert-type = "Sequent Dev CA"`.
- **Other CA** — matches certificates with `cert-type = "Other CA"`.

Each classifier subflow contains:

1. A **Condition - auth note** step that checks `cert-type` equals the expected CA CN (e.g. `"Sequent Dev CA"`).
2. An **X509/Validate Username Form** step (`x509-user-resolution`) configured to match voters by SubjectDN using the regex `(?:CN=)([^,]*)`.

### Adding a new CA subflow

For each new CA imported in the Certificates tab, add a corresponding conditional subflow in the **Credentials subflow**:

1. In the Keycloak admin console, navigate to the election event's realm → **Authentication** → **certificate browser flow** → **Credentials subflow**.
2. Add a new **conditional subflow** (e.g. named after the CA).
3. Inside the new subflow, add:
   - A **Condition - auth note** step with `cert-type` = the CN of the new CA (set `value-is-regex` to `false`).
   - An **X509/Validate Username Form** step configured to match voters by SubjectDN.
4. Update the **No classiffier match** subflow's condition: edit the `NoClassifierMatch condition` authenticator config and add the new CA's CN to the negated regex. For example, if the existing regex is `(Sequent Dev CA)|(Other CA)`, extend it to `(Sequent Dev CA)|(Other CA)|(New CA CN)`.

:::tip
The `NoClassifierMatch condition` uses a negated regex. Any certificate whose `cert-type` does not match the regex will trigger the deny-access authenticator, resulting in an `accessDenied` error. Keep this regex in sync with all configured classifier subflows.
:::

---

## Error cases

The **Deny subflow** handles three error scenarios. Voters will see the corresponding message on the login page:

| Error key | Message | Cause |
|---|---|---|
| `certNotProvided` | Authentication failed: No certificate was provided | The voter did not present a client certificate. |
| `userNotFound` | Authentication failed: Valid certificate detected, but no matching voter or user was found | The certificate was valid and classified, but no voter record matched. |
| `accessDenied` | Authentication failed: Invalid certificate, access denied | The certificate was not classified by any subflow (issuer CA not configured or not trusted). |

These messages are defined in the Keycloak theme translation files at:

```
packages/keycloak-extensions/sequent-theme/src/main/resources/theme/sequent.voting-portal/login/messages/messages_<lang>.properties
```

They can be overridden per-realm in the Keycloak admin console under **Realm Settings** → **Localization**.
