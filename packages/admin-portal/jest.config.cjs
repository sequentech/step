// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** @type {import('jest').Config} */
module.exports = {
    collectCoverageFrom: ["src/**/*.{ts,tsx}", "!src/**/*.d.ts", "!src/**/*.test.{ts,tsx}"],
    coverageProvider: "babel",
    coverageReporters: ["text", "html", "lcov", "json", "json-summary"],
    testEnvironment: "node",
    testMatch: ["<rootDir>/src/**/*.test.{ts,tsx}"],
    moduleNameMapper: {
        "^@/(.*)$": "<rootDir>/src/$1",
        "^@sequentech/ui-core$": "<rootDir>/../ui-core/src/types/VotingChannel.ts",
    },
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
