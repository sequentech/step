// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import config from "../workbench/eslint.config.mjs"

export default [
    ...config,
    {ignores: ["src/kc.gen.tsx", "dist_keycloak/**", "public/keycloakify-dev-resources/**"]},
]
