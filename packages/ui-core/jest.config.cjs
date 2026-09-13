// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/** @type {import('jest').Config} */
module.exports = {
    testEnvironment: "node",
    testMatch: ["<rootDir>/src/**/*.test.ts", "<rootDir>/src/**/*.test.tsx"],
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
    // Include unimported runtime files. Tests cannot raise the score merely by
    // avoiding modules; only declarations and test files are outside this scope.
    collectCoverageFrom: ["src/**/*.{ts,tsx}", "!src/**/*.d.ts", "!src/**/*.test.{ts,tsx}"],
    // Instrument source before Babel creates CommonJS re-export getters. V8
    // would otherwise count those generated branches against this package.
    coverageProvider: "babel",
    coverageDirectory: "coverage",
    coverageReporters: ["text", "html", "lcov", "json", "json-summary"],
    coverageThreshold: {
        global: {lines: 95, statements: 95, functions: 95, branches: 95},
    },
}
