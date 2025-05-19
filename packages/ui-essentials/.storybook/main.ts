// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
const webpack = require("webpack")
const path = require("path")

module.exports = {
    stories: ["../src/**/*.mdx", "../src/**/*.stories.mdx", "../src/**/*.stories.@(js|jsx|ts|tsx)"],
    addons: [
        "@storybook/addon-links",
        "@storybook/addon-essentials",
        "@storybook/addon-interactions",
        "@storybook/preset-create-react-app",
        "storybook-addon-react-router-v6",
        "@storybook/addon-mdx-gfm",
        "storybook-addon-pseudo-states",
    ],
    framework: {
        name: "@storybook/react-webpack5",
        options: {},
    },
    features: {
        interactionsDebugger: true, // 👈 Enable playback controls
    },

    port: 9009,
    docs: {
        autodocs: true,
    },

    // 🧩 Add this to enable WebAssembly
    webpackFinal: async (config) => {
        config.experiments = {
            ...config.experiments,
            asyncWebAssembly: true,
        }

        config.plugins.push(
            new webpack.BannerPlugin({
                banner: `SPDX-FileCopyrightText: 2025 Enric Badia <enric@xtremis.com>
SPDX-License-Identifier: AGPL-3.0-only`,
                raw: true,
                entryOnly: true,
            })
        )

        return config
    },
}
