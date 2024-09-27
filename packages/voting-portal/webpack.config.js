// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const path = require("path")

const {CleanWebpackPlugin} = require("clean-webpack-plugin")
const ESLintPlugin = require("eslint-webpack-plugin")
const {ProgressPlugin} = require("webpack")
const HtmlWebpackPlugin = require("html-webpack-plugin")
const CopyWebpackPlugin = require("copy-webpack-plugin")

class InterpolateHtmlPlugin {
    // Replaces %VARIABLE% with the corresponding variable from the replacements object
    constructor(replacements) {
        this.replacements = replacements
    }

    apply(compiler) {
        compiler.hooks.compilation.tap("InterpolateHtmlPlugin", (compilation) => {
            HtmlWebpackPlugin.getHooks(compilation).beforeEmit.tapAsync(
                "InterpolateHtmlPlugin", // The name of this plugin
                (data, cb) => {
                    // Use the variable values to replace the placeholders
                    Object.keys(this.replacements).forEach((key) => {
                        const value = this.replacements[key]
                        // Create a global regular expression to find all instances
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
            publicPath: "/", // Set to empty string to ensure correct base path
        },
        devtool: "source-map",
        module: {
            rules: [
                {
                    test: /\.css$/i,
                    include: path.resolve(__dirname, "src"),
                    use: ["style-loader", "css-loader", "postcss-loader"],
                },
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
        externals: {},
        resolve: {
            alias: {
                "@root": path.resolve(__dirname, "src"),
                "@": path.resolve(__dirname, "src"),
            },
            extensions: [".js", ".jsx", ".ts", ".tsx"],
        },
        plugins: [
            new InterpolateHtmlPlugin({
                PUBLIC_URL: "", // Provide replacements for variables
            }),
            new HtmlWebpackPlugin({
                template: path.resolve(__dirname, "public/index.html"),
                favicon: path.resolve(__dirname, "public/favicon.ico"),
                filename: "./index.html",
                favicon: "./public/favicon.ico",
                // pass variables to the template
                templateParameters: {
                    PUBLIC_URL: "", // Replace %PUBLIC_URL% with an empty string
                },
            }),

            // Configure CopyWebpackPlugin to include a list of files from 'public/' into 'dist/'
            new CopyWebpackPlugin({
                patterns: [
                    {
                        from: path.resolve(__dirname, "public"), // Source folder
                        to: path.resolve(__dirname, "dist"), // Destination folder
                        globOptions: {
                            ignore: [
                                // Ignore all .html and .ico files (as examples, you can modify as needed)
                                "**/index.html",
                                "**/favicon.ico",
                            ],
                        },
                    },
                ],
            }),
            new ProgressPlugin(),
            new ESLintPlugin({
                extensions: [".js", ".jsx", ".ts", ".tsx"],
            }),
            new CleanWebpackPlugin(),
        ],
        devServer: {
            static: {
                directory: path.resolve(__dirname, "dist"),
            },
            compress: true, // Enable gzip compression
            port: 3000, // Run on port 3000
            open: true, // Automatically open the browser
            historyApiFallback: true,
        },
    }
}
