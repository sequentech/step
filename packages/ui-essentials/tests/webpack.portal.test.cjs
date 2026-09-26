// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const os = require("os")
const path = require("path")
const {withPortalDevelopment} = require("../webpack.portal.cjs")

const PACKAGES = path.resolve(__dirname, "../..")
const PORTAL = path.join(PACKAGES, "voting-portal")
const UI_CORE_SOURCE = path.join(PACKAGES, "ui-core", "src")
const UI_ESSENTIALS_SOURCE = path.join(PACKAGES, "ui-essentials", "src")
const ENVIRONMENT = ["STEP_SHARED_UI"]

const portalConfig = (mode) => ({
    mode,
    entry: path.join(PORTAL, "src", "index.tsx"),
    devtool: "source-map",
    module: {
        rules: [
            {test: /\.css$/i, use: ["style-loader", "css-loader"]},
            {
                test: /\.(js|ts)x?$/,
                use: [
                    "babel-loader",
                    {loader: "ts-loader", options: {configFile: "tsconfig.json"}},
                ],
            },
        ],
    },
    resolve: {alias: {"@root": path.join(PORTAL, "src")}},
    plugins: [],
    devServer: {port: 3000, open: true, historyApiFallback: true},
})

const pluginNamed = (config, name) =>
    config.plugins.find((plugin) => plugin.constructor.name === name)

// Runs the plugin's beforeResolve hook for one import and returns the context used.
function resolveContext(config, issuer, request) {
    let beforeResolve
    const factory = {hooks: {beforeResolve: {tap: (_name, hook) => (beforeResolve = hook)}}}
    pluginNamed(config, "PortalInstancesPlugin").apply({
        hooks: {normalModuleFactory: {tap: (_name, hook) => hook(factory)}},
    })
    const data = {request, context: path.dirname(issuer), contextInfo: {issuer}}
    beforeResolve(data)
    return data.context
}

const saved = {}
beforeEach(() => {
    ENVIRONMENT.forEach((name) => {
        saved[name] = process.env[name]
        delete process.env[name]
    })
})
afterEach(() => {
    ENVIRONMENT.forEach((name) => {
        if (saved[name] === undefined) delete process.env[name]
        else process.env[name] = saved[name]
    })
})

describe("production", () => {
    it("keeps the package entries, loaders, source maps and plugins", () => {
        const config = portalConfig("production")
        const result = withPortalDevelopment(PORTAL, config)
        expect(result.resolve).toBe(config.resolve)
        expect(result.module).toBe(config.module)
        expect(result.plugins).toBe(config.plugins)
        expect(result.devtool).toBe("source-map")
        expect(result.devServer).toEqual(config.devServer)
    })

    it("ignores STEP_SHARED_UI", () => {
        process.env.STEP_SHARED_UI = "unknown"
        expect(withPortalDevelopment(PORTAL, portalConfig("production")).module.rules).toHaveLength(
            2
        )
    })
})

describe("development", () => {
    it("resolves only the package entries to their sources", () => {
        const {alias} = withPortalDevelopment(PORTAL, portalConfig("development")).resolve
        expect(alias).toEqual({
            "@root": path.join(PORTAL, "src"),
            "@sequentech/ui-core$": path.join(UI_CORE_SOURCE, "index.tsx"),
            "@sequentech/ui-essentials$": path.join(UI_ESSENTIALS_SOURCE, "index.tsx"),
        })
    })

    it("scopes each package's @root alias to its own sources", () => {
        const {rules} = withPortalDevelopment(PORTAL, portalConfig("development")).module
        expect(rules.slice(2)).toEqual([
            {include: UI_CORE_SOURCE, resolve: {alias: {"@root": UI_CORE_SOURCE}}},
            {include: UI_ESSENTIALS_SOURCE, resolve: {alias: {"@root": UI_ESSENTIALS_SOURCE}}},
        ])
    })

    it("transpiles without type checking and adds the React Refresh transform", () => {
        const config = withPortalDevelopment(PORTAL, portalConfig("development"))
        const [styles, scripts] = config.module.rules
        expect(styles).toEqual(portalConfig("development").module.rules[0])
        expect(scripts.use).toEqual([
            {
                loader: "babel-loader",
                options: {
                    plugins: [[require.resolve("react-refresh/babel"), {skipEnvCheck: true}]],
                },
            },
            {loader: "ts-loader", options: {configFile: "tsconfig.json", transpileOnly: true}},
        ])
        expect(config.devtool).toBe("eval-cheap-module-source-map")
    })

    it("keeps the portal entry out of React Refresh", () => {
        const config = portalConfig("development")
        const refresh = pluginNamed(withPortalDevelopment(PORTAL, config), "ReactRefreshPlugin")
        expect(refresh.options.overlay).toBe(false)
        expect(refresh.options.exclude).toEqual([/node_modules/, config.entry])
    })

    it("keeps the built packages with STEP_SHARED_UI=dist", () => {
        process.env.STEP_SHARED_UI = "dist"
        const config = withPortalDevelopment(PORTAL, portalConfig("development"))
        expect(config.resolve.alias).toEqual({"@root": path.join(PORTAL, "src")})
        expect(config.module.rules).toHaveLength(2)
        expect(pluginNamed(config, "PortalInstancesPlugin")).toBeUndefined()
        expect(pluginNamed(config, "ReactRefreshPlugin")).toBeDefined()
    })

    it("rejects an unknown STEP_SHARED_UI", () => {
        process.env.STEP_SHARED_UI = "source"
        expect(() => withPortalDevelopment(PORTAL, portalConfig("development"))).toThrow(
            "STEP_SHARED_UI must be one of: src, dist"
        )
    })
})

describe("shared instances", () => {
    const config = () => withPortalDevelopment(PORTAL, portalConfig("development"))
    const header = path.join(UI_ESSENTIALS_SOURCE, "components", "Header", "Header.tsx")
    const i18n = path.join(UI_CORE_SOURCE, "services", "i18n.ts")

    it.each([
        ["Header", "react/jsx-runtime", header],
        ["Header", "@mui/material/styles", header],
        ["ui-core i18n", "i18next", i18n],
        ["ui-core i18n", "sequent-core", i18n],
    ])("resolves the %s import of %s from the portal", (_label, request, issuer) => {
        expect(resolveContext(config(), issuer, request)).toBe(PORTAL)
    })

    it.each([
        ["Header", "lodash", header],
        ["Header", "./HeaderLogo", header],
        ["Header", "@mui/icons-material/Logout", header],
        ["portal App", "react", path.join(PORTAL, "src", "App.tsx")],
        ["entry", "react", ""],
    ])("leaves the %s import of %s to the importing workspace", (_label, request, issuer) => {
        expect(resolveContext(config(), issuer, request)).toBe(path.dirname(issuer))
    })

    it("leaves packages the portal cannot resolve to the shared UI workspace", () => {
        const outside = withPortalDevelopment(os.tmpdir(), portalConfig("development"))
        expect(resolveContext(outside, header, "react")).toBe(path.dirname(header))
    })
})
