#!/usr/bin/env node

// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

// SPDX-License-Identifier: AGPL-3.0-only

import * as esbuild from 'esbuild'

await esbuild.build({
  entryPoints: ['app/content/keycloak-man/KeycloakManLovesJsx.tsx'],
  bundle: true,
  format: "esm",
  packages: "external",
  loader: { '.tsx': 'tsx' },
  outdir: '../resources/content/keycloak-man',
})