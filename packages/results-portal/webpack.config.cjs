// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const path = require("path")

const ESLintPlugin = require("eslint-webpack-plugin")
const {ProgressPlugin} = require("webpack")
const HtmlWebpackPlugin = require("html-webpack-plugin")
const CopyWebpackPlugin = require("copy-webpack-plugin")

class InterpolateHtmlPlugin {
    constructor(replacements) {
        this.replacements = replacements
    }

    apply(compiler) {
        compiler.hooks.compilation.tap("InterpolateHtmlPlugin", (compilation) => {
            HtmlWebpackPlugin.getHooks(compilation).beforeEmit.tapAsync(
                "InterpolateHtmlPlugin",
                (data, cb) => {
                    Object.keys(this.replacements).forEach((key) => {
                        const value = this.replacements[key]
                        data.html = data.html.replace(new RegExp(`%${key}%`, "g"), value)
                    })
                    cb(null, data)
                }
            )
        })
    }
}

module.exports = function (env, argv) {
    return {
        mode: argv.mode,
        entry: path.resolve(__dirname, "src/index.tsx"),
        output: {
            filename: "index.js",
            path: path.resolve(__dirname, "dist"),
            publicPath: "/",
            clean: true,
        },
        devtool: "source-map",
        module: {
            rules: [
                {
                    test: /\.css$/i,
                    include: [
                        path.resolve(__dirname, "src"),
                        path.resolve(__dirname, "../node_modules/@mui/x-data-grid"),
                    ],
                    use: ["style-loader", "css-loader", "postcss-loader"],
                },
                {
                    test: /\.(js|ts)x?$/,
                    exclude: /node_modules/,
                    use: [
                        "babel-loader",
                        {
                            loader: "ts-loader",
                            options: {
                                transpileOnly: true,
                            },
                        },
                    ],
                },
                {
                    type: "asset",
                    test: /\.(png|jpe?g|gif|ico|svg|wasm)$/i,
                },
            ],
        },
        resolve: {
            fallback: {
                fs: false,
                path: false,
                crypto: false,
            },
            alias: {
                "@root": path.resolve(__dirname, "src"),
                "@": path.resolve(__dirname, "src"),
            },
            extensions: [".js", ".jsx", ".ts", ".tsx"],
        },
        plugins: [
            new InterpolateHtmlPlugin({
                PUBLIC_URL: "",
            }),
            new HtmlWebpackPlugin({
                template: path.resolve(__dirname, "public/index.html"),
                favicon: path.resolve(__dirname, "public/favicon.ico"),
                filename: "./index.html",
                templateParameters: {
                    PUBLIC_URL: "",
                },
            }),
            new CopyWebpackPlugin({
                patterns: [
                    {
                        from: path.resolve(__dirname, "public"),
                        to: path.resolve(__dirname, "dist"),
                        globOptions: {
                            ignore: ["**/index.html", "**/favicon.ico"],
                        },
                    },
                ],
            }),
            new ProgressPlugin(),
            new ESLintPlugin(),
        ],
        devServer: {
            static: {
                directory: path.resolve(__dirname, "dist"),
            },
            compress: true,
            port: Number(process.env.PORT || 3004),
            open: true,
            historyApiFallback: true,
        },
    }
}
