// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const path = require("path")

const {CleanWebpackPlugin} = require("clean-webpack-plugin")
const ESLintPlugin = require("eslint-webpack-plugin")
const {ProgressPlugin} = require("webpack")

module.exports = function (env, argv) {
    return {
        mode: argv.mode,
        entry: path.resolve(__dirname, "src/index.tsx"),
        output: {
            filename: "index.js",
            library: "$",
            libraryTarget: "umd",
            path: path.resolve(__dirname, "dist")
        },
        devtool: "source-map",
        module: {
            rules: [
                {
                    test: /\.(js|ts)x?$/,
                    exclude: /node_modules/,
                    use: [
                        "babel-loader",
                        "ts-loader",
                    ],
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
            "@sequentech/ui-core": "@sequentech/ui-core",
        },
        resolve: {
            alias: {
                "@root": path.resolve(__dirname, "src"),
            },
            extensions: [".js", ".jsx", ".ts", ".tsx", ".d.ts"],
        },
        plugins: [
            new ProgressPlugin(),
            new ESLintPlugin({
                extensions: [".js", ".jsx", ".ts", ".tsx"],
            }),
            new CleanWebpackPlugin()
        ],
    }
}
