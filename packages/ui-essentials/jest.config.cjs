// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** @type {import("jest").Config} */
module.exports = {
    testMatch: [
        "<rootDir>/src/**/*.test.ts",
        "<rootDir>/src/**/*.test.tsx",
        "<rootDir>/tests/*.test.cjs",
    ],
    // Measure executable source, including components no test imports.
    collectCoverageFrom: [
        "src/**/*.{ts,tsx}",
        "!src/**/*.d.ts",
        "!src/**/*.test.{ts,tsx}",
        "!src/**/__stories__/**",
        "!src/**/*.stories.{ts,tsx}",
        "!src/testing/**",
    ],
    coverageProvider: "babel",
    coverageReporters: ["text", "html", "lcov", "json", "json-summary"],
    // CI compares every measured metric against the PR base.
    //
    // jsdom by default, so a component can be interacted with rather than only
    // stringified: the ballot (`Question`, `Answer`, `AnswersList`,
    // `InvalidErrorsList`) lives here, shared with the Election Architect's
    // preview. Node-only tests opt out with a `@jest-environment node` docblock.
    testEnvironment: "jsdom",
    setupFiles: ["<rootDir>/tests/setupGlobals.ts"],
    setupFilesAfterEnv: ["<rootDir>/src/testing/setup.ts"],
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
    moduleNameMapper: {
        // Stylesheets are a bundler's side effect; jest hands them to its
        // JavaScript parser and reports a SyntaxError from inside a dependency.
        "\\.(css|less|scss|sass)$": "<rootDir>/src/testing/styleStub.ts",

        // `ui-core` by source, not through its unbuilt `dist/index.js`. Its own
        // files resolve through a tsconfig alias jest does not read, so that is
        // mapped too — otherwise the failure names `@root/types/LanguageConf`,
        // a module nobody wrote.
        "^@sequentech/ui-core$": "<rootDir>/../ui-core/src/index.tsx",
        "^@root/(.*)$": "<rootDir>/../ui-core/src/$1",

        // The WASM package is ESM resolving its binary through
        // `new URL(…, import.meta.url)`, which jest's transform cannot load. It
        // is the boundary the ballot's engine will be injected at, so stubbing it
        // here is the same seam, one release early.
        "^sequent-core$": "<rootDir>/src/testing/sequentCoreStub.ts",
    },
}
