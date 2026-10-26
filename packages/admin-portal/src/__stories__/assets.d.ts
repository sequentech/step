// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The portal's GraphQL introspection, which .storybook/main.ts bundles for
// schema-checked story boundaries.
declare module "virtual:admin-graphql-schema" {
    const introspection: string
    export default introspection
}

// Public CA fixture text used by certificate-settings interaction stories.
declare module "*.pem?raw" {
    const pem: string
    export default pem
}
