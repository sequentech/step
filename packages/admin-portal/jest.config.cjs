// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** @type {import('jest').Config} */
module.exports = {
    collectCoverageFrom: [
        "src/**/*.{ts,tsx}",
        "!src/**/*.d.ts",
        "!src/**/*.test.{ts,tsx}",
        "!src/**/*.stories.{ts,tsx}",
        "!src/**/__stories__/**",
    ],
    coverageProvider: "babel",
    coverageReporters: ["text", "html", "lcov", "json", "json-summary"],
    testEnvironment: "node",
    testMatch: ["<rootDir>/src/**/*.test.{ts,tsx}"],
    setupFiles: ["<rootDir>/src/test/polyfills.cjs"],
    // @vitrine/ui exports its elements only under the "import" condition,
    // which Jest's CommonJS resolver doesn't match.
    moduleNameMapper: {
        "^@/(.*)$": "<rootDir>/src/$1",
        "^@sequentech/ui-core$": "<rootDir>/../ui-core/src/types/VotingChannel.ts",
        "^@vitrine/ui$": "<rootDir>/../node_modules/@vitrine/ui/dist/index.js",
        "^@vitrine/ui/(receipt|status|proof-steps|chain|alert|badge)$":
            "<rootDir>/../node_modules/@vitrine/ui/dist/$1.js",
    },
    // The vendored Vitrine packages and @noble, which @vitrine/verify uses, ship
    // only ES modules, so they are transformed like the app's own code.
    transformIgnorePatterns: ["/node_modules/(?!(@vitrine|@noble)/)"],
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
