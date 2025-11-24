// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const path = require("path")

const ESLintPlugin = require("eslint-webpack-plugin")
const {ProgressPlugin} = require("webpack")

module.exports = function (env, argv) {
    return {
        mode: argv.mode,
        entry: path.resolve(__dirname, "src/index.tsx"),
        experiments: {
            outputModule: true,
            asyncWebAssembly: true,
        },
        output: {
            filename: "index.js",
            library: {
                type: "module", // <-- ES module, no UMD bootstrap
            },
            path: path.resolve(__dirname, "dist"),
            clean: true,
        },
        devtool: "source-map",
        module: {
            rules: [
                {
                    test: /\.(js|ts)x?$/,
                    exclude: /node_modules/,
                    use: ["babel-loader", "ts-loader"],
                },
                {
                    type: "asset",
                    test: /\.(png|jpe?g|gif|ico|svg)$/i,
                },
            ],
        },
        externals: {
            "react": "react",
            "react-dom": "react-dom",
            "@mui/material": "@mui/material",
            "@emotion/react": "@emotion/react",
            "@emotion/styled": "@emotion/styled",
            "mui-image": "mui-image",
            "react-i18next": "react-i18next",
            "react-router-dom": "react-router-dom",
            "sequent-core": "sequent-core",
        },
        resolve: {
            alias: {
                "@root": path.resolve(__dirname, "src"),
            },
            extensions: [".js", ".jsx", ".ts", ".tsx", ".wasm"],
        },
        plugins: [
            new ProgressPlugin(),
            new ESLintPlugin({
                extensions: [".js", ".jsx", ".ts", ".tsx"],
            }),
        ],
    }
}
