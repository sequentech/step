// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
module.exports = {
    stories: ["../src/**/*.stories.mdx", "../src/**/*.stories.@(js|jsx|ts|tsx)"],
    addons: [
        "@storybook/addon-links",
        "@storybook/addon-essentials",
        "@storybook/addon-interactions",
        "@storybook/preset-create-react-app",
        "storybook-addon-react-router-v6",
        "@storybook/addon-mdx-gfm",
    ],
    framework: {
        name: "@storybook/react-webpack5",
        options: {},
    },
    features: {
        interactionsDebugger: true, // 👈 Enable playback controls
    },

    port: 9009,
}
