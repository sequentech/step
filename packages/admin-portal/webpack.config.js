const path = require("path")

const {CleanWebpackPlugin} = require("clean-webpack-plugin")
const ESLintPlugin = require("eslint-webpack-plugin")
const {ProgressPlugin} = require("webpack")
const HtmlWebpackPlugin = require('html-webpack-plugin')

module.exports = function (env, argv) {
    return {
        mode: argv.mode,
        entry: path.resolve(__dirname, "src/index.tsx"),
        output: {
            filename: "index.js",
            path: path.resolve(__dirname, "dist"),
            publicPath: '', // Set to empty string to ensure correct base path
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
        externals: {
        },
        resolve: {
            alias: {
                "@root": path.resolve(__dirname, "src"),
                "@": path.resolve(__dirname, "src"),
            },
            extensions: [".js", ".jsx", ".ts", ".tsx"],
        },
        plugins: [
            new HtmlWebpackPlugin({
                template: './public/index.html',
                filename: './index.html',
                favicon: './public/favicon.ico',
                publicPath: '', // Set to empty string to remove %PUBLIC_URL%
                // pass variables to the template
                templateParameters: {
                    'PUBLIC_URL': '' // Replace %PUBLIC_URL% with an empty string
                }
            }),
            new ProgressPlugin(),
            new ESLintPlugin({
                extensions: [".js", ".jsx", ".ts", ".tsx"],
            }),
            new CleanWebpackPlugin(),
        ],
        devServer: {
            contentBase: './dist',
            // no publicPath
        },
    }
}
