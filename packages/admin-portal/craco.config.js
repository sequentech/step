// SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const CracoAlias = require("craco-alias")

module.exports = {
    devServer: {
        headers: {
            "Access-Control-Allow-Origin": "*",
            "Access-Control-Allow-Methods": "*",
            "Access-Control-Allow-Headers": "*",
            "Access-Control-Allow-Credentials": "true",
            "Cross-Origin-Resource-Policy": "cross-origin",
            "Referrer-Policy": "no-referrer",
            "Cross-Origin-Embedder-Policy": "credentialless",
        },
    },
    webpack: {
        alias: {
            "react/jsx-runtime.js": "react/jsx-runtime",
            "react/jsx-dev-runtime.js": "react/jsx-dev-runtime",
        },
    },
    plugins: [
        {
            plugin: CracoAlias,
            options: {
                source: "tsconfig",
                baseUrl: ".",
                tsConfigPath: "./tsconfig.json",
            },
        },
    ],
}
