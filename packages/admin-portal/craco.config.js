// SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
const CracoAlias = require("craco-alias")
const path = require("path")

module.exports = {
    devServer: (devServerConfig, {env, paths}) => {
        devServerConfig.headers = {
            "Access-Control-Allow-Origin": "*",
            "Access-Control-Allow-Methods": "*",
            "Access-Control-Allow-Headers": "*",
            "Access-Control-Allow-Credentials": "true",
            "Cross-Origin-Resource-Policy": "cross-origin",
            "Referrer-Policy": "no-referrer",
            "Cross-Origin-Embedder-Policy": "credentialless",
        };
        
        // Remove deprecated properties for webpack-dev-server v5 compatibility
        delete devServerConfig.onAfterSetupMiddleware;
        delete devServerConfig.onBeforeSetupMiddleware;
        delete devServerConfig.https;
        
        // Use the new setupMiddlewares instead
        devServerConfig.setupMiddlewares = (middlewares, devServer) => {
            return middlewares;
        };
        
        // Configure HTTPS properly for webpack-dev-server v5
        if (process.env.HTTPS === 'true') {
            devServerConfig.server = 'https';
        }
        
        return devServerConfig;
    },
    webpack: {
        alias: {
            "react/jsx-runtime.js": "react/jsx-runtime",
            "react/jsx-dev-runtime.js": "react/jsx-dev-runtime",
        },
        configure: (webpackConfig, {env, paths}) => {
            // Handle hoisted dependencies in monorepo
            webpackConfig.resolve.modules = [
                ...webpackConfig.resolve.modules,
                // Add the correct path where sequent-core actually is
                path.resolve(__dirname, "../node_modules"),
            ]

            // Enable WebAssembly support
            webpackConfig.experiments = {
                ...webpackConfig.experiments,
                asyncWebAssembly: true,
            }

            webpackConfig.resolve.fallback = {
                ...webpackConfig.resolve.fallback,
                fs: false,
                path: false,
                crypto: false,
            }

            return webpackConfig
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
