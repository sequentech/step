// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
module.exports = {
    moduleNameMapper: {
        "^@sequentech/ui-core$": "<rootDir>/../ui-core/src/index.tsx",
        "^@sequentech/ui-essentials$": "<rootDir>/../ui-essentials/src/index.tsx",
    },
    // This profile exercises service, store and provider contracts. Browser app
    // routing is a separate scope; src/App.test.tsx is an obsolete CRA scaffold.
    testMatch: [
        "<rootDir>/src/services/**/*.test.ts",
        "<rootDir>/src/store/**/*.test.ts",
        "<rootDir>/src/providers/**/*.test.tsx",
    ],
    collectCoverageFrom: [
        "src/**/*.{ts,tsx}",
        "!src/**/*.d.ts",
        "!src/**/*.test.{ts,tsx}",
        "!src/setupTests.ts",
        "!src/stories/**",
    ],
    coverageProvider: "babel",
    coverageReporters: ["text", "html", "lcov", "json", "json-summary"],
    testEnvironment: "node",
    setupFilesAfterEnv: ["<rootDir>/src/setupTests.ts"],
    transform: {
        "^.+\\.[jt]sx?$": [
            "babel-jest",
            {
                presets: [
                    ["@babel/preset-env", {targets: {node: "current"}}],
                    ["@babel/preset-react", {runtime: "automatic"}],
                    "@babel/preset-typescript",
                ],
            },
        ],
    },
}
