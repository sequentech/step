---
id: keycloak-config
title: Keycloak Configuration
format: md
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Keycloak Configuration

The Keycloak-related configuration required for the IVR is mostly bootstrapped automatically and doesn't require any
manual operations. But this document explains how the system works, and what is the required setup if anything goes
wrong and manual intervention is required.

## Setup

For IVR to function, two dedicated clients are required on Keycloak: `ivr-service` and `ivr-voting`.
The former is used by the IVR handler to perform voter-independent tasks, such as reading the IVR auth
configuration or checking the blacklist.
The latter is used as a proxy client to authenticate as the voter through IVR-sourced credentials, such as voter id and
pin.
Both clients should be created in IVR-enabled election event realms and configured as described below.

## Service Client (`ivr-service`)

Service client is a service account for IVR internal use. It doesn't have a special client ID, but it is named
`ivr-service` by default.

Its client ID and secret are configured on the IVR handler during deployment. If either changes, another deployment is
required. Because a single deployment can serve multiple election events, all realms should have the same client id and
secret.

### Required Configuration:

1. **Auth config**: `ivr-config-read` realm role must be assigned for reading the IVR auth steps configuration.
2. **Blacklisting**: The `ivr-service` service account user must have `tenant-id` attribute set, and
   `phone-blacklist-read` realm role assigned.
3. **Auth method**: `Service account roles` auth flow should be enabled, `Client ID` and `Client Secret` credentials
   should be acquired for deployment.

## Voting Client (`ivr-voting`)

Voting client is a dedicated, well-known client with ID `ivr-voting` for authenticating as the voter. The specific
client ID is used by auth mechanisms and for optional auth flow override.

1. **Auth method**: `Direct access grants` auth flow should be enabled, `Client ID` and `Client Secret` credentials
   should be acquired for deployment.