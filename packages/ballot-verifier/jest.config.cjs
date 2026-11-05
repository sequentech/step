// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
/** @type {import('jest').Config} */
module.exports = {
    moduleNameMapper: {
        "^@sequentech/ui-core$": "<rootDir>/../ui-core/src/index.tsx",
        "^@sequentech/ui-essentials$": "<rootDir>/../ui-essentials/src/index.tsx",
        // The WASM bindings are an ES module around a .wasm binary, which Jest
        // cannot load. Tests answer for it through src/__mocks__/sequentCore.ts;
        // the real module runs in the Storybook stories and Playwright journeys.
        "^sequent-core$": "<rootDir>/src/__mocks__/sequentCore.ts",
        "\\.(css|png|svg)$": "<rootDir>/src/__mocks__/staticAsset.ts",
        // @vitrine/ui exports its elements only under the "import" condition,
        // which Jest's CommonJS resolver doesn't match.
        "^@vitrine/ui$": "<rootDir>/../node_modules/@vitrine/ui/dist/index.js",
        "^@vitrine/ui/(receipt|status|proof-steps|chain|alert|badge)$":
            "<rootDir>/../node_modules/@vitrine/ui/dist/$1.js",
    },
    // The vendored Vitrine packages and @noble, which @vitrine/verify uses, ship
    // only ES modules, so they are transformed like the app's own code.
    transformIgnorePatterns: ["/node_modules/(?!(@vitrine|@noble)/)"],
    // src/App.test.tsx is an obsolete Create React App scaffold, and coverage
    // runs apply this list to the base revision too, so App tests use another name.
    testMatch: [
        "<rootDir>/src/services/**/*.test.ts",
        "<rootDir>/src/store/**/*.test.ts",
        "<rootDir>/src/providers/**/*.test.tsx",
        "<rootDir>/src/screens/**/*.test.tsx",
        "<rootDir>/src/App.routing.test.tsx",
    ],
    collectCoverageFrom: [
        "src/**/*.{ts,tsx}",
        "!src/**/*.d.ts",
        "!src/**/*.test.{ts,tsx}",
        "!src/__mocks__/**",
        "!src/setupTests.ts",
        "!src/setupJestGlobals.ts",
        "!src/stories/**",
        "!src/**/*.stories.{ts,tsx}",
        "!src/**/__stories__/**",
    ],
    coverageProvider: "babel",
    coverageReporters: ["text", "html", "lcov", "json", "json-summary"],
    // Screens are asserted through the accessibility tree, which needs a DOM.
    // A fixed ?lang=en keeps i18n from falling back to browser detection.
    testEnvironment: "jsdom",
    testEnvironmentOptions: {url: "http://localhost/?lang=en"},
    setupFiles: ["<rootDir>/src/setupJestGlobals.ts"],
    setupFilesAfterEnv: ["<rootDir>/src/setupTests.ts"],
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
