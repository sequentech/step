// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

module.exports = {
    collectCoverageFrom: ["src/**/*.{ts,tsx}", "!src/**/*.d.ts", "!src/**/*.test.{ts,tsx}"],
    coverageProvider: "babel",
    coverageReporters: ["text", "html", "lcov", "json", "json-summary"],
    moduleNameMapper: {
        "^@sequentech/ui-core$": "<rootDir>/../ui-core/src/index.tsx",
        "^@sequentech/ui-essentials$": "<rootDir>/../ui-essentials/src/index.tsx",
        "^@/(.*)$": "<rootDir>/src/$1",
    },
    testEnvironment: "node",
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
