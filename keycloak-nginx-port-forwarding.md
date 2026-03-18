<!--
SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Keycloak nginx port forwarding in dev

## Design

Everything goes through nginx — both standard login AND cert login.

- `"keycloak-nginx:8443"` in `forwardPorts` → VS Code assigns local:8443 (since it's the only claimant)
- `KC_MTLS_LOGIN_URL=https://127.0.0.1:8443` → cert button hits nginx on 8443
- Standard Keycloak redirect to `https://127.0.0.1:8443` also hits nginx → nginx proxies to Keycloak:8090
- No conflict, no separate 8444 needed

Keycloak also listens on 8443 internally but it is not mapped in docker-compose `ports:` (only 8090 is),
so it is unreachable from outside docker. The only 8443 forwarded to the local machine is keycloak-nginx's.

## How VS Code port forwarding actually works here

VS Code's `forwardPorts` with `"service:port"` notation uses the **docker-compose host port
mapping** as the local port — it does NOT tunnel directly into the container bypassing host
mappings. So `"keycloak-nginx:8443"` with `"8444:8443"` in docker-compose → `localhost:8444`.

This devcontainer uses `docker-outside-of-docker`: docker runs on the Codespaces VM host, the
devcontainer is a sibling container. VS Code runs inside the devcontainer and can only forward
devcontainer-level ports. Manually adding port 8444 in the VS Code ports tab results in a
timeout (that host port is on the VM host network, not inside the devcontainer). But the
automatic `forwardPorts` mechanism DOES work — it just picks the host-mapped port as the local
port.

The fix is therefore to align the host port in `docker-compose.yml` with the expected local
port: `"8443:8443"` instead of `"8444:8443"`.

## Current state (2026-03-18)

### Problem investigated
`keycloak-nginx:8443` in `forwardPorts` was forwarding to `localhost:8444` instead of
`localhost:8443`, so `KC_MTLS_LOGIN_URL=https://127.0.0.1:8443` did not reach nginx.

### Root cause
`docker-compose.yml` had `"8444:8443"` for keycloak-nginx. VS Code's `forwardPorts` uses the
host-mapped port (8444) as the local port, not the container port (8443).

### Fix applied
Changed `docker-compose.yml` keycloak-nginx port mapping from `"8444:8443"` to `"8443:8443"`.

### After restart, the VS Code ports tab should show:
- `keycloak-nginx:8443 | localhost:8443` — nginx, used for both standard login and cert button
- `keycloak:8090 | localhost:8090` — Keycloak HTTP (internal use by nginx proxy_pass)

Do NOT manually add `keycloak:8443` to the ports tab.
