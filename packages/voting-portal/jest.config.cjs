// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** @type {import('jest').Config} */
module.exports = {
    testEnvironment: "node",
    testMatch: ["<rootDir>/src/**/*.test.ts"],
    // @sequentech/ui-core resolves to its built dist/ bundle, which a clean
    // `yarn install` doesn't produce; see src/__mocks__/uiCoreTestEntry.ts.
    moduleNameMapper: {
        "^@sequentech/ui-core$": "<rootDir>/src/__mocks__/uiCoreTestEntry.ts",
    },
    transform: {
        "^.+\\.(t|j)sx?$": [
            "@swc/jest",
            {
                jsc: {
                    parser: {syntax: "typescript", tsx: true},
                    target: "es2022",
                },
                module: {type: "commonjs"},
            },
        ],
    },
}
