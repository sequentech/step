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
    // @sequentech/ui-core and @sequentech/ui-essentials resolve to their built
    // dist/ bundles, which a clean `yarn install` doesn't produce; see
    // src/__mocks__/uiCoreTestEntry.ts and src/__mocks__/uiEssentialsTestEntry.ts.
    moduleNameMapper: {
        "^@sequentech/ui-core$": "<rootDir>/src/__mocks__/uiCoreTestEntry.ts",
        "^@sequentech/ui-essentials$": "<rootDir>/src/__mocks__/uiEssentialsTestEntry.ts",
        // The WASM package can't be loaded by jest; see src/__mocks__/sequentCoreStub.ts.
        "^sequent-core$": "<rootDir>/src/__mocks__/sequentCoreStub.ts",
    },
    transform: {
        "^.+\\.(t|j)sx?$": [
            "@swc/jest",
            {
                jsc: {
                    parser: {syntax: "typescript", tsx: true},
                    target: "es2022",
                    transform: {react: {runtime: "automatic"}},
                },
                module: {type: "commonjs"},
            },
        ],
    },
}
