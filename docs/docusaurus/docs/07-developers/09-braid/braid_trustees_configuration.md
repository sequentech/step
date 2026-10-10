---
id: braid_trustees_configuration
title: Wbraid trustee configuration
sidebar_label: Wbraid trustees
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The native `packages/trustee` runner uses the protocol and cryptography supplied
by `packages/wbraid`. Each trustee keeps a signing key and a share encryption
key locally. The administration portal registers both public keys.

Generate a new identity once, then print its public keys when registering it:

```sh
trustee generate --trustee-config trustee.toml
trustee public-keys --trustee-config trustee.toml
trustee start --trustee-config trustee.toml
```

Generation refuses to replace an existing file. Back up `trustee.toml` and the
runner's `message_store` directory. The runner does not generate replacement
keys when its configured identity is missing.

| Variable | Purpose |
| --- | --- |
| `TRUSTEE_CONFIG_PATH` | Local native keys file, unless supplied with `--trustee-config` |
| `B4_URL` | Native board URL, unless supplied with `--b4-url` |
| `TRUSTEE_HARVEST_URL` | Platform API that lists this trustee's assigned boards |
| `TRUSTEE_NAME`, `TRUSTEE_PASSWORD` | Trustee's Keycloak account |
| `TRUSTEE_KEYCLOAK_URL`, `TRUSTEE_KEYCLOAK_REALM` | Tenant's authentication server and realm |
| `TRUSTEE_KEYCLOAK_CLIENT_ID`, `TRUSTEE_KEYCLOAK_CLIENT_SECRET` | Authentication client; omit the secret for a public client |
| `LOG_LEVEL` | Runner tracing filter |

The runner discovers DKG and tally boards through Harvest, verifies signed
configuration and message lineage, and resumes its local protocol sessions.
Containers use `http://b4v6:3005` for the supplied native board service.

For manual ceremonies, download the public custody request from the portal and
produce a signed proof locally:

```sh
trustee attest --request custody-request.json --trustee-config trustee.toml --output custody-proof.json
```

Upload the proof. For a tally restore, add `--tally-session-id <tally UUID>`.
The proof is bound to the event, ceremony, trustee and, for restore, tally.
Private keys stay in the runner's keys file.

The ballot context is the signed configuration's immutable decimal identifier.
Export/import can remap platform UUIDs and board names while preserving signed
canonical messages and that cryptographic identifier.
