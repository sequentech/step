// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

module.exports = {
    typescript: {
        enableTypeChecking: false,
    },
    webpack: {
        configure: (webpackConfig) => {
            // Find and configure ts-loader to skip type checking
            const tsLoader = webpackConfig.module.rules.find(
                (rule) =>
                    rule.oneOf && rule.oneOf.find((r) => r.loader && r.loader.includes("ts-loader"))
            )
            if (tsLoader && tsLoader.oneOf) {
                const tsRule = tsLoader.oneOf.find(
                    (r) => r.loader && r.loader.includes("ts-loader")
                )
                if (tsRule && !tsRule.options) {
                    tsRule.options = {}
                }
                if (tsRule && tsRule.options) {
                    tsRule.options.transpileOnly = true
                }
            }
            return webpackConfig
        },
    },
    devServer: (devServerConfig, {env, paths}) => {
        devServerConfig.headers = {
            "Access-Control-Allow-Origin": "*",
            "Access-Control-Allow-Methods": "*",
            "Access-Control-Allow-Headers": "*",
            "Access-Control-Allow-Credentials": "true",
            "Cross-Origin-Resource-Policy": "cross-origin",
            "Cross-Origin-Embedder-Policy": "credentialless",
            "Referrer-Policy": "no-referrer",
        }

        // Remove deprecated properties for webpack-dev-server v5 compatibility
        delete devServerConfig.onAfterSetupMiddleware
        delete devServerConfig.onBeforeSetupMiddleware
        delete devServerConfig.https

        // Use the new setupMiddlewares instead
        devServerConfig.setupMiddlewares = (middlewares, devServer) => {
            return middlewares
        }

        // Configure HTTPS properly for webpack-dev-server v5
        if (process.env.HTTPS === "true") {
            devServerConfig.server = "https"
        }

        return devServerConfig
    },
}
