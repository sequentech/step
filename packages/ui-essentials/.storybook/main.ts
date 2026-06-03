// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {StorybookConfig} from "@storybook/react-vite"
import {mergeConfig} from "vite"
import path from "path"

const config: StorybookConfig = {
    stories: ["../src/**/*.mdx", "../src/**/*.stories.@(js|jsx|ts|tsx)"],
    addons: [
        "@storybook/addon-links",
        "@storybook/addon-essentials",
        "@storybook/addon-interactions",
        "@storybook/addon-docs",
        "storybook-addon-remix-react-router",
        "storybook-addon-pseudo-states",
        "@storybook/addon-viewport",
    ],
    framework: {
        name: "@storybook/react-vite",
        options: {},
    },
    docs: {
        autodocs: true,
        defaultName: "Docs",
    },

    typescript: {
        reactDocgen: "react-docgen-typescript",
        reactDocgenTypescriptOptions: {
            shouldExtractLiteralValuesFromEnum: true,
            shouldRemoveUndefinedFromOptional: true,
            propFilter: (prop) => {
                if (prop.parent) {
                    return (
                        !prop.parent.fileName.includes("node_modules") &&
                        !prop.parent.fileName.includes("/dist/")
                    )
                }
                return true
            },
        },
    },

    async viteFinal(viteConfig, {configType}) {
        return mergeConfig(viteConfig, {
            resolve: {
                alias: [{find: "@root", replacement: path.resolve(__dirname, "../src")}],
            },

            build: {
                sourcemap: configType === "DEVELOPMENT",
            },

            optimizeDeps: {
                exclude: ["sequent-core"],
            },
        })
    },
}

export default config
