---
id: cli
title: Step CLI
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

`step-cli` administers elections through authenticated APIs. `step-cli load` prepares synthetic voting workloads and runs them with k6 or Chromium.

## Build in the devcontainer

From the repository root:

```bash
devenv shell
export CARGO_TARGET_DIR="$PWD/packages/step-cli/rust-local-target"
cargo build \
  --manifest-path packages/step-cli/Cargo.toml \
  --bin step-cli
export PATH="$CARGO_TARGET_DIR/debug:$PATH"
step-cli load \
  --help
```

The binary includes Rust coordination, native encryption, SQLite aggregation, SVG/HTML reporting and election fixtures. HTTP workloads require only k6 alongside the CLI; the repository devenv provides it. Browser runs and report screenshots additionally need Node.js, `@playwright/test` and its matching Chromium installation. The devcontainer provides Chromium, which initialization discovers on `PATH`.

For browser workloads or report screenshots outside devenv, install Playwright:

```bash
npm install \
  --prefix .load-browser \
  --save-exact @playwright/test@1.62.1
.load-browser/node_modules/.bin/playwright install \
  --with-deps chromium
```

Set `runtime.playwright_dir` to the absolute `.load-browser` path in your workload. Install k6 in `PATH`, or set `runtime.k6` to its executable. `load check` reports missing dependencies before election provisioning.

## Authenticate a tenant administrator

Use an existing tenant reserved for synthetic voters, an administrator account, and a Keycloak client enabled for CLI authentication. The client belongs to the tenant's administrative realm. Obtain its ID and secret from the deployment operator; a public client uses an empty secret.

```bash
umask 077
read -r -p 'Synthetic tenant ID: ' TENANT_ID
read -r -p 'GraphQL URL: ' GRAPHQL_URL
read -r -p 'Keycloak base URL: ' KEYCLOAK_URL
read -r -p 'Administrator username: ' ADMIN_USER
read -rs -p 'Administrator password: ' ADMIN_PASSWORD
read -r -p 'CLI client ID: ' CLIENT_ID
read -rs -p 'CLI client secret: ' CLIENT_SECRET

step-cli step config \
  --tenant-id "$TENANT_ID" \
  --endpoint-url "$GRAPHQL_URL" \
  --keycloak-url "$KEYCLOAK_URL" \
  --keycloak-user "$ADMIN_USER" \
  --keycloak-password "$ADMIN_PASSWORD" \
  --keycloak-client-id "$CLIENT_ID" \
  --keycloak-client-secret "$CLIENT_SECRET"
unset ADMIN_PASSWORD CLIENT_SECRET
```

The local devcontainer URLs are `http://graphql-engine:8080/v1/graphql` and `http://keycloak:8090`. The CLI stores its session in `config/configuration.json` beside the executable; that directory must be writable and kept private.

Before automatic election setup, check the tenant's registered trustees:

```bash
step-cli step list-trustees
```

If registration is needed, obtain each running trustee service's name and public key, then register it:

```bash
read -r -p 'Trustee service name: ' TRUSTEE_NAME
read -r -p 'Trustee public key (base64): ' TRUSTEE_PUBLIC_KEY
step-cli step create-trustee \
  --name "$TRUSTEE_NAME" \
  --public-key "$TRUSTEE_PUBLIC_KEY"
```

Register enough running trustees for the configured ceremony threshold. Continue with the [voting load quickstart](../05-voting-portal/voter-status-performance.md).

## Reference and development

Use `step-cli load reference` for configuration defaults and command options. The [generated reference](../05-voting-portal/voting-load-reference.md) is produced from the CLI help and configuration rustdoc.

```bash
step-cli load reference \
  --output docs/docusaurus/docs/07-developers/05-voting-portal/voting-load-reference.md
cargo test \
  --manifest-path packages/step-cli/Cargo.toml
cargo doc \
  --manifest-path packages/step-cli/Cargo.toml \
  --no-deps \
  --document-private-items
```
