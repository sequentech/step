// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {StorybookConfig} from "@storybook/react-vite"
import {fileURLToPath} from "node:url"
import {resolve} from "node:path"
import {mergeConfig} from "vite"
import {sequentCoreViteAlias} from "../../ui-core/sequent-core-dev.cjs"

const sourceEntry = (workspace: string) =>
    fileURLToPath(new URL(`../../${workspace}/src/index.tsx`, import.meta.url))

const entryConfigDir = fileURLToPath(new URL(".", import.meta.url))

/** Portal Storybooks composed into this one; an empty URL variable leaves a portal out. */
const PORTAL_REFS = [
    {id: "voting-portal", title: "Voting portal", env: "STORYBOOK_VOTING_PORTAL_URL", port: 6007},
    {id: "admin-portal", title: "Admin portal", env: "STORYBOOK_ADMIN_PORTAL_URL", port: 6008},
    {
        id: "results-portal",
        title: "Results portal",
        env: "STORYBOOK_RESULTS_PORTAL_URL",
        port: 6009,
    },
    {
        id: "ballot-verifier",
        title: "Ballot verifier",
        env: "STORYBOOK_BALLOT_VERIFIER_URL",
        port: 6010,
    },
] as const

const portalRefs = () =>
    Object.fromEntries(
        PORTAL_REFS.flatMap(({id, title, env, port}) => {
            const url = process.env[env] ?? `http://localhost:${port}`
            return url ? [[id, {title, url}]] : []
        })
    )

const config: StorybookConfig = {
    stories: ["../src/**/*.mdx", "../src/**/*.stories.tsx"],
    // Portal Storybooks spread this configuration: only the development server of
    // this entry Storybook composes them, and a static build stays self-contained.
    refs: (refs, {configDir, configType}) =>
        configType === "DEVELOPMENT" && resolve(configDir) === resolve(entryConfigDir)
            ? {...refs, ...portalRefs()}
            : refs,
    staticDirs: ["../public"],
    addons: [
        "@storybook/addon-docs",
        "@storybook/addon-a11y",
        "@storybook/addon-vitest",
        "storybook-addon-pseudo-states",
    ],
    framework: "@storybook/react-vite",
    core: {disableTelemetry: true},
    typescript: {
        reactDocgen: "react-docgen-typescript",
        reactDocgenTypescriptOptions: {
            shouldExtractLiteralValuesFromEnum: true,
            shouldRemoveUndefinedFromOptional: true,
            propFilter: (prop) =>
                !prop.parent ||
                (!prop.parent.fileName.includes("node_modules") &&
                    !prop.parent.fileName.includes("/dist/")),
        },
    },
    viteFinal: (viteConfig) =>
        mergeConfig(viteConfig, {
            resolve: {
                dedupe: ["react", "react-dom"],
                // Exact match: only the package entry moves to its source.
                alias: [
                    {
                        find: /^@\//,
                        replacement: `${resolve(viteConfig.root ?? process.cwd(), "src")}/`,
                    },
                    {
                        find: /^@root\//,
                        replacement: `${resolve(viteConfig.root ?? process.cwd(), "src")}/`,
                    },
                    {find: /^@sequentech\/ui-core$/, replacement: sourceEntry("ui-core")},
                    {
                        find: /^@sequentech\/ui-essentials$/,
                        replacement: sourceEntry("ui-essentials"),
                    },
                    ...sequentCoreViteAlias(),
                ],
            },
            // sequent-core fetches its .wasm relative to its own module URL.
            optimizeDeps: {
                // Nightwatch brings Vue test-utils into this React workspace;
                // Vitest otherwise discovers it and requires absent Vue peers.
                exclude: ["sequent-core", "@vue/test-utils"],
            },
        }),
}

export default config
