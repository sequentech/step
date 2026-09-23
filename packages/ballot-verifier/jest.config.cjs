// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** @type {import('jest').Config} */
module.exports = {
    testEnvironment: "jsdom",
    testMatch: ["<rootDir>/src/**/*.test.ts", "<rootDir>/src/**/*.test.tsx"],
    setupFiles: ["<rootDir>/src/test/polyfills.cjs"],
    setupFilesAfterEnv: ["<rootDir>/src/setupTests.ts"],
    moduleNameMapper: {
        "^@sequentech/ui-core$": "<rootDir>/../ui-core/src/index.tsx",
        "^@sequentech/ui-essentials$": "<rootDir>/../ui-essentials/src/index.tsx",
        "\\.(png|jpg|jpeg|gif|svg)$": "<rootDir>/src/test/fileMock.cjs",
        "\\.(css|less|scss)$": "<rootDir>/src/test/styleMock.cjs",
    },
    collectCoverageFrom: [
        "src/**/*.{ts,tsx}",
        "!src/**/*.d.ts",
        "!src/**/*.test.{ts,tsx}",
        "!src/setupTests.ts",
        "!src/stories/**",
    ],
    coverageProvider: "babel",
    coverageReporters: ["text", "html", "lcov", "json", "json-summary"],
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
