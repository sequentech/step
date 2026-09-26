// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Settings shared by the portal webpack configs. In development the shared UI
// packages compile from their sources inside each portal, so an edit reaches the
// browser through HMR without a package build. Production builds keep consuming
// the packages' `dist` entry points.

const fs = require("fs")
const path = require("path")
const ReactRefreshWebpackPlugin = require("@pmmmwh/react-refresh-webpack-plugin")

const PACKAGES = path.resolve(__dirname, "..")
const SHARED_UI = ["ui-core", "ui-essentials"].map((name) => ({
    request: `@sequentech/${name}`,
    source: path.join(PACKAGES, name, "src"),
}))

// `STEP_SHARED_UI=dist` keeps the built packages in development too, e.g. to
// reproduce a difference that only the production bundles show.
const SharedUiEntry = Object.freeze({SOURCE: "src", BUILD: "dist"})

// Shared UI source resolves packages from its own workspace. These hold React,
// theme, router, i18n, Apollo or WASM state, so they must be the portal's copies.
const PORTAL_INSTANCES = [
    "react",
    "react-dom",
    "react-router",
    "react-router-dom",
    "react-i18next",
    "i18next",
    "@emotion/react",
    "@emotion/styled",
    "@mui/material",
    "@mui/system",
    "@mui/x-data-grid",
    "@apollo/client",
    "sequent-core",
]

function sharedUiEntry() {
    const entry = process.env.STEP_SHARED_UI ?? SharedUiEntry.SOURCE
    const entries = Object.values(SharedUiEntry)
    if (!entries.includes(entry)) {
        throw new Error(`STEP_SHARED_UI must be one of: ${entries.join(", ")}`)
    }
    return entry
}

const packageName = (request) => request.split("/", request.startsWith("@") ? 2 : 1).join("/")

const isSharedUiSource = (file) => SHARED_UI.some(({source}) => file.startsWith(source + path.sep))

function isInstalledFrom(directory, name) {
    const parent = path.dirname(directory)
    return (
        fs.existsSync(path.join(directory, "node_modules", name)) ||
        (parent !== directory && isInstalledFrom(parent, name))
    )
}

class PortalInstancesPlugin {
    constructor(portal) {
        this.portal = portal
        this.names = new Set(PORTAL_INSTANCES.filter((name) => isInstalledFrom(portal, name)))
    }

    apply(compiler) {
        compiler.hooks.normalModuleFactory.tap(PortalInstancesPlugin.name, (factory) => {
            factory.hooks.beforeResolve.tap(PortalInstancesPlugin.name, (data) => {
                if (
                    this.names.has(packageName(data.request)) &&
                    isSharedUiSource(data.contextInfo.issuer)
                ) {
                    data.context = this.portal
                }
            })
        })
    }
}

function withSharedUiSource(portal, config) {
    return {
        ...config,
        module: {
            ...config.module,
            rules: [
                ...config.module.rules,
                // The packages' own `@root` alias, as in their webpack and tsconfig.
                ...SHARED_UI.map(({source}) => ({
                    include: source,
                    resolve: {alias: {"@root": source}},
                })),
            ],
        },
        resolve: {
            ...config.resolve,
            alias: {
                ...config.resolve.alias,
                // Exact match: subpaths such as `/public/*.svg` keep the package.
                ...Object.fromEntries(
                    SHARED_UI.map(({request, source}) => [
                        `${request}$`,
                        path.join(source, "index.tsx"),
                    ])
                ),
            },
        },
        plugins: [...config.plugins, new PortalInstancesPlugin(portal)],
    }
}

const loaderName = (entry) => (typeof entry === "string" ? entry : entry.loader)

// Type errors are reported by the typecheck scripts and production builds, off the
// edit-to-browser path. The webpack mode, not NODE_ENV, selects React Refresh.
function developmentLoader(entry) {
    const {loader, options = {}} = typeof entry === "string" ? {loader: entry} : entry
    switch (loader) {
        case "ts-loader":
            return {loader, options: {...options, transpileOnly: true}}
        case "babel-loader":
            return {
                loader,
                options: {
                    ...options,
                    plugins: [
                        ...(options.plugins ?? []),
                        [require.resolve("react-refresh/babel"), {skipEnvCheck: true}],
                    ],
                },
            }
        default:
            return entry
    }
}

/**
 * Completes a portal config. In development mode the shared UI packages resolve
 * to source, compiled by the portal rule that uses ts-loader, and React Refresh
 * keeps component state across edits of component-only modules.
 */
function withPortalDevelopment(portal, config) {
    if (config.mode !== "development") {
        return config
    }
    const development = {
        ...config,
        module: {
            ...config.module,
            rules: config.module.rules.map((rule) =>
                Array.isArray(rule.use) && rule.use.map(loaderName).includes("ts-loader")
                    ? {...rule, use: rule.use.map(developmentLoader)}
                    : rule
            ),
        },
        plugins: [
            ...config.plugins,
            // The entry creates the React root: an update reaching it reloads the
            // page instead of mounting a second root.
            new ReactRefreshWebpackPlugin({
                overlay: false,
                exclude: [/node_modules/, config.entry],
            }),
        ],
    }
    return sharedUiEntry() === SharedUiEntry.SOURCE
        ? withSharedUiSource(portal, development)
        : development
}

module.exports = {withPortalDevelopment}
