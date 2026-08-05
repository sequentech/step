// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** @type {import('jest').Config} */
module.exports = {
    testEnvironment: "node",
    testMatch: ["<rootDir>/src/**/*.test.ts"],
    // The real sequent-core WASM package is plain ESM and gets pulled in
    // transitively by @sequentech/ui-core; see src/__mocks__/sequentCoreMock.js.
    moduleNameMapper: {
        "^sequent-core$": "<rootDir>/src/__mocks__/sequentCoreMock.js",
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
