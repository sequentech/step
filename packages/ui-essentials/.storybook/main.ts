// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {StorybookConfig} from "@storybook/react-vite"
import {fileURLToPath} from "node:url"
import {mergeConfig} from "vite"

const sourceEntry = (workspace: string) =>
    fileURLToPath(new URL(`../../${workspace}/src/index.tsx`, import.meta.url))

const config: StorybookConfig = {
    stories: ["../src/**/*.mdx", "../src/**/*.stories.tsx"],
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
                // Exact match: only the package entry moves to its source.
                alias: [{find: /^@sequentech\/ui-core$/, replacement: sourceEntry("ui-core")}],
            },
            // sequent-core fetches its .wasm relative to its own module URL.
            optimizeDeps: {exclude: ["sequent-core"]},
        }),
}

export default config
