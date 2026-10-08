---
id: admin_portal_tutorials_multi_idp_attribute_linking
title: Linking Multiple IdP Identities to a Single User via Custom Attribute
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Overview

By default, Keycloak's identity brokering links an external Identity Provider (IdP) user to a
Keycloak user on a 1-to-1 basis, usually by matching `email` or `username`. This tutorial
explains how to configure the **Custom Attribute IdP Identity Linking** authenticator so that
multiple IdP identities (e.g., different emails or subject IDs from the same or different IdPs)
can be mapped to a single Keycloak user.

The authenticator reads a configurable claim from the incoming IdP identity and searches for a
Keycloak user whose **multi-value custom attribute** contains that value. When exactly one match
is found the new IdP identity is linked to the existing user automatically.

---

## Prerequisites

- Access to the Keycloak Admin Console.
- The `sequent.idp-linking-authenticator.jar` extension deployed in Keycloak's `providers/`
  directory (included in the Sequent Keycloak Docker image by default).
- An existing External Identity Provider configured in the target realm.

---

## Step 1 – Create the Custom User Attribute

The authenticator searches for users by a Keycloak user-profile attribute. You must create this
attribute before configuring the flow.

1. In the Keycloak Admin Console, select the realm you want to configure.
2. Navigate to **Realm settings** → **User profile** → **Create attribute**.
3. Set the **Name** to `linked_idp_identities` (or any name you will use in the authenticator
   configuration).
4. Enable the attribute **multi-valued** (do not restrict it to a single value).
5. Optionally restrict read/write permissions so only administrators can manage it.
6. Click **Save**.

> **Tip:** After creating the attribute you can pre-populate it for existing users via the Admin
> Console (**Users** → select user → **Attributes** tab) or via the Admin REST API.

---

## Step 2 – Create the First Broker Login Flow

In this step we will create `sequent first broker login multivalue` flow directly.

1. Navigate to **Authentication** → **Flows**.
2. Click on create flow
3. Give the new flow a descriptive name such as `sequent first broker login multivalue`.

---

## Step 3 – Add the Custom Authenticator to the Flow

1. Open the duplicnewated flow.
2. Click **Add step** inside the appropriate sub-flow.
3. Search for **Custom Attribute IdP Identity Linking** and add it.
4. Set the requirement to **REQUIRED** if you want the flow to
   fail when no match is found or **ALTERNATIVE** (the authenticator will call `attempted()` when no
   matching user is found, allowing the next step to run, if you fant to create a user if not found for example).
5. Click **⚙ Config** (gear icon) next to the new step to configure it:

   | Parameter | Description | Default |
   |---|---|---|
   | **IdP Claim** | Claim/attribute name to read from the incoming IdP identity. Use well-known names (`email`, `username`, `id`/`sub`, `firstname`, `lastname`) or a custom mapped attribute (e.g., `SAFE_ID`). | `email` |
   | **User Attribute** | Keycloak user attribute (multi-value) to search for the claim value. | `linked_idp_identities` |

6. Click **Save**.

---

## Step 4 – Bind the New Flow to the Identity Provider

1. Navigate to **Identity Providers** and select the IdP you want to configure.
2. In the **First Login Flow** (or **First Broker Login Flow**) dropdown, select the duplicated
   flow you created in Step 2.
3. Click **Save**.

---

## Step 5 – Map the Custom Claim from the IdP Token

If the claim you want to use (e.g., `SAFE_ID`) is not a standard OIDC/SAML field, add an
attribute mapper on the IdP:

1. In the IdP configuration, open the **Mappers** tab.
2. Click **Add mapper**.
3. Set **Mapper type** to **Attribute Importer** (for OIDC) or **SAML Attribute** (for SAML).
4. Map the IdP claim name to the Keycloak attribute name that matches what you set in
   **IdP Claim** (e.g., `SAFE_ID`).
5. Click **Save**.

---

## Behavior Summary

| Scenario | Authenticator action |
|---|---|
| Configuration is missing | Passes to the next step (`attempted`). |
| The IdP claim is empty or absent | Passes to the next step (`attempted`). |
| No user found with the attribute value | Passes to the next step (`attempted`). |
| Exactly one user found | Links the IdP identity to that user and succeeds. |
| More than one user found | Fails the flow with `IDENTITY_PROVIDER_ERROR` to prevent ambiguous linking. |
