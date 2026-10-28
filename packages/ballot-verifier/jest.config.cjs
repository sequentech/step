// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** @type {import('jest').Config} */
module.exports = {
    testEnvironment: "jsdom",
    testMatch: ["<rootDir>/src/**/*.test.ts", "<rootDir>/src/**/*.test.tsx"],
    setupFiles: ["<rootDir>/src/test/polyfills.cjs"],
    setupFilesAfterEnv: ["<rootDir>/src/setupTests.ts"],
    // @vitrine/ui exports its elements only under the "import" condition,
    // which Jest's CommonJS resolver doesn't match.
    moduleNameMapper: {
        "\\.(png|jpg|jpeg|gif|svg)$": "<rootDir>/src/test/fileMock.cjs",
        "\\.(css|less|scss)$": "<rootDir>/src/test/styleMock.cjs",
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
