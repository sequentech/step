// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** @type {import('jest').Config} */
module.exports = {
    // Component tests assert on the accessibility tree (role, accessible name,
    // focus), none of which exists without a DOM.
    testEnvironment: "jsdom",
    testMatch: ["<rootDir>/src/**/*.test.ts", "<rootDir>/src/**/*.test.tsx"],
    setupFiles: ["<rootDir>/src/setupJestGlobals.ts"],
    setupFilesAfterEnv: ["<rootDir>/src/setupTests.ts"],
    // @sequentech/ui-core resolves to its built dist/ bundle, which a clean
    // `yarn install` doesn't produce; see src/__mocks__/uiCoreTestEntry.ts.
    moduleNameMapper: {
        "^@sequentech/ui-core$": "<rootDir>/src/__mocks__/uiCoreTestEntry.ts",
    },
    // Unimported runtime files remain in the denominator. Only Jest support
    // and tests are excluded; production fixtures and entry points stay.
    collectCoverageFrom: [
        "src/**/*.{ts,tsx}",
        "!src/**/*.d.ts",
        "!src/**/*.test.{ts,tsx}",
        "!src/__mocks__/**",
        "!src/setupTests.ts",
        "!src/setupJestGlobals.ts",
    ],
    coverageProvider: "babel",
    coverageDirectory: "coverage",
    coverageReporters: ["text", "html", "lcov", "json", "json-summary"],
    coverageThreshold: {global: {lines: 95, statements: 95, functions: 95, branches: 95}},
    transform: {
        "^.+\\.[jt]sx?$": [
            "babel-jest",
            {
                babelrc: false,
                configFile: false,
                presets: [
                    ["@babel/preset-env", {targets: {node: "current"}}],
                    ["@babel/preset-react", {runtime: "automatic"}],
                    "@babel/preset-typescript",
                ],
            },
        ],
    },
}
