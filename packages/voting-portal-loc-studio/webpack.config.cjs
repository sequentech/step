// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const path = require("path")

const packagesRoot = path.resolve(__dirname, "..")
const {ProgressPlugin} = require(path.join(packagesRoot, "node_modules/webpack"))
const HtmlWebpackPlugin = require(path.join(packagesRoot, "node_modules/html-webpack-plugin"))
const CopyWebpackPlugin = require(path.join(packagesRoot, "node_modules/copy-webpack-plugin"))

const votingPortalSrc = path.join(packagesRoot, "voting-portal/src")

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

module.exports = function (_env, argv) {
    return {
        mode: argv.mode,
        entry: path.resolve(__dirname, "src/standalone/index.tsx"),
        experiments: {
            asyncWebAssembly: true,
        },
        output: {
            filename: "loc-studio.js",
            path: path.resolve(__dirname, "dist"),
            publicPath: "/",
            clean: true,
        },
        devtool: "source-map",
        module: {
            rules: [
                {
                    test: /\.css$/i,
                    use: ["style-loader", "css-loader"],
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
                    test: /\.(png|jpe?g|gif|ico|svg)$/i,
                },
            ],
        },
        resolve: {
            modules: [
                path.join(packagesRoot, "node_modules"),
                path.join(__dirname, "node_modules"),
                "node_modules",
            ],
            alias: {
                "@voting-portal": votingPortalSrc,
                "@root": votingPortalSrc,
                "@": votingPortalSrc,
            },
            extensions: [".js", ".jsx", ".ts", ".tsx", ".wasm"],
        },
        plugins: [
            new InterpolateHtmlPlugin({
                PUBLIC_URL: "",
            }),
            new HtmlWebpackPlugin({
                template: path.resolve(__dirname, "public/loc-studio.html"),
                favicon: path.resolve(__dirname, "public/favicon.ico"),
                filename: "./index.html",
                templateParameters: {
                    PUBLIC_URL: "",
                },
            }),
            new CopyWebpackPlugin({
                patterns: [
                    {
                        from: path.resolve(__dirname, "public/demo-banner.png"),
                        to: path.resolve(__dirname, "dist/demo-banner.png"),
                        noErrorOnMissing: true,
                    },
                ],
            }),
            new ProgressPlugin(),
        ],
        devServer: {
            static: {
                directory: path.resolve(__dirname, "dist"),
            },
            compress: true,
            port: Number(process.env.PORT) || 3010,
            open: true,
            historyApiFallback: true,
            hot: true,
            client: {
                overlay: {
                    errors: true,
                    warnings: false,
                },
            },
        },
    }
}
