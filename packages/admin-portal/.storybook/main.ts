// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {StorybookConfig} from "@storybook/react-vite"
import {readFileSync} from "node:fs"
import {createRequire} from "node:module"
import {dirname} from "node:path"
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
                include: [
                    "ra-i18n-polyglot",
                    "@mui/icons-material/ChevronRight",
                    "@mui/material/Checkbox",
                    "@mui/icons-material/CalendarMonth",
                    "@mui/icons-material/Cached",
                    "@mui/icons-material/CheckCircle",
                    "@mui/icons-material/Key",
                    "jotai",
                    "@mui/icons-material/ViewColumn",
                    "@mui/icons-material/InfoOutlined",
                    "@mui/icons-material/Assignment",
                    "sql.js",
                    "idb",
                    "ra-data-hasura",
                    "react-admin-json-view",
                    "@mui/material/Tab",
                    "@mui/icons-material/Edit",
                    "@mui/icons-material/Delete",
                    "moment-timezone",
                    "@apollo/client/link/context",
                    "@mui/icons-material/Add",
                    "@mui/icons-material/Publish",
                    "@mui/icons-material/Lock",
                    "@mui/icons-material/NoEncryptionGmailerrorred",
                    "@mui/icons-material/Description",
                    "@mui/icons-material/Preview",
                    "react-use",
                    "braid-wasm",
                    "@mui/icons-material/Mail",
                    "@mui/icons-material/CreditScore",
                    "@mui/icons-material/Password",
                    "@mui/icons-material/Article",
                    "@mui/icons-material/FilterAlt",
                    "@mui/icons-material/SyncAlt",
                    "uuid",
                    "@mui/material/Tabs",
                    "react-hook-form",
                    "json-edit-react",
                    "lodash/get",
                    "@mui/icons-material/Group",
                    "@mui/icons-material/Settings",
                    "@mui/icons-material/Help",
                    "react-js-cron",
                    "date-fns",
                    "@mui/icons-material/Search",
                    "@mui/icons-material/Web",
                    "@tinymce/tinymce-react",
                    "@mui/icons-material/Fence",
                    "@mui/icons-material/MarkEmailReadOutlined",
                    "@mui/icons-material/SmsOutlined",
                    "@mui/icons-material/OpenInNew",
                    "@mui/icons-material/Unpublished",
                    "@mui/icons-material/PublishedWithChanges",
                    "@mui/material/TextField",
                    "@mui/icons-material/VisibilityOffOutlined",
                    "@mui/icons-material/VisibilityOutlined",
                    "@mui/icons-material/Clear",
                    "intl-tel-input/react",
                    "lodash/isEqual",
                    "diff",
                    "@mui/icons-material/CancelOutlined",
                    "@mui/icons-material/CheckCircleOutline",
                    "@mui/icons-material/MoreHoriz",
                    "@mui/icons-material/AddCircle",
                    "@mui/icons-material/Inventory",
                    "@emotion/react",
                    "@mui/icons-material/HowToVote",
                    "@mui/icons-material/DragIndicator",

                    "ra-language-english",
                    "graphql",
                    "keycloak-js",
                    "@mui/icons-material/Download",
                    "@mui/icons-material/Upload",
                    "@mui/icons-material/ExpandMore",
                    "@mui/icons-material/Close",
                    "@mui/icons-material/ContentCopy",
                    "@mui/icons-material/Visibility",
                ],
            },
        }),
} satisfies StorybookConfig

export default adminConfig
