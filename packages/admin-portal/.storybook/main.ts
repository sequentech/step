// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {StorybookConfig} from "@storybook/react-vite"
import {readFileSync, readdirSync} from "node:fs"
import {createRequire} from "node:module"
import {dirname, join} from "node:path"
import {fileURLToPath} from "node:url"
import config from "../../ui-essentials/.storybook/main.ts"
import postcssPresetEnv from "postcss-preset-env"
import {mergeConfig, searchForWorkspaceRoot, type Plugin} from "vite"

// Role stories read the default tenant groups from the Keycloak realm template.
const realmTemplates = fileURLToPath(
    new URL("../../../.devcontainer/keycloak/import", import.meta.url)
)

// Story GraphQL boundaries check operations against the portal's schema. A
// module of the story bundle provides it, so no story requests it at runtime.
const SCHEMA_MODULE = "virtual:admin-graphql-schema"
const adminGraphqlSchema = (): Plugin => ({
    name: "admin-graphql-schema",
    resolveId: (id) => (id === SCHEMA_MODULE ? `\0${SCHEMA_MODULE}` : undefined),
    load: (id) =>
        id === `\0${SCHEMA_MODULE}`
            ? `export default ${JSON.stringify(
                  readFileSync(new URL("../graphql.schema.json", import.meta.url), "utf8")
              )}`
            : undefined,
})

// As webpack.config.cjs does: braid's threaded WASM loads its unbundled modules
// from /braid-wasm and needs a cross-origin isolated page for shared memory.
const braidWasm = dirname(createRequire(import.meta.url).resolve("braid-wasm/package.json"))
const CROSS_ORIGIN_ISOLATION = {
    "Cross-Origin-Opener-Policy": "same-origin",
    "Cross-Origin-Embedder-Policy": "require-corp",
    "Cross-Origin-Resource-Policy": "cross-origin",
}

// Vite pre-bundles each deep MUI import separately; one first reached while
// stories run reloads the page and fails the stories in flight. Every deep MUI
// import in the portal's source is known before the run.
const sources = fileURLToPath(new URL("../src", import.meta.url))
const MUI_DEEP_IMPORT = /from "(@mui\/(?:icons-material|material)\/[A-Za-z]+)"/g
const muiDeepImports = [
    ...new Set(
        readdirSync(sources, {recursive: true, encoding: "utf8"})
            .filter((file) => /\.tsx?$/.test(file))
            .flatMap((file) =>
                [...readFileSync(join(sources, file), "utf8").matchAll(MUI_DEEP_IMPORT)].map(
                    ([, specifier]) => specifier
                )
            )
    ),
].sort()

const adminConfig = {
    ...config,
    staticDirs: ["../public", {from: braidWasm, to: "/braid-wasm"}],
    viteFinal: async (viteConfig, options) =>
        mergeConfig(await config.viteFinal!(viteConfig, options), {
            plugins: [adminGraphqlSchema()],
            // As in webpack.config.cjs: public assets such as /tinymce are served from the root.
            define: {"process.env.MAX_DIFF_LINES": "500", "process.env.PUBLIC_URL": '""'},
            server: {
                fs: {allow: [searchForWorkspaceRoot(process.cwd()), realmTemplates]},
                headers: CROSS_ORIGIN_ISOLATION,
            },
            css: {postcss: {plugins: [postcssPresetEnv()]}},
            optimizeDeps: {
                // When Vite finds a dependency while stories run, it bundles it
                // and reloads the page, failing the stories in flight. Scanning
                // every story file first finds the packages they reach.
                entries: ["src/**/*.stories.tsx"],
                include: [
                    ...muiDeepImports,
                    "ra-i18n-polyglot",
                    "jotai",
                    "sql.js",
                    "idb",
                    "ra-data-hasura",
                    "react-admin-json-view",
                    "moment-timezone",
                    "@apollo/client/link/context",
                    "react-use",
                    "braid-wasm",
                    "uuid",
                    "react-hook-form",
                    "json-edit-react",
                    "lodash/get",
                    "react-js-cron",
                    "date-fns",
                    "@tinymce/tinymce-react",
                    "intl-tel-input/react",
                    "lodash/isEqual",
                    "diff",
                    "@emotion/react",

                    "ra-language-english",
                    "graphql",
                    "keycloak-js",
                ],
            },
        }),
} satisfies StorybookConfig

export default adminConfig
