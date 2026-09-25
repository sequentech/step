// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
module.exports = {
    moduleNameMapper: {
        "^@sequentech/ui-core$": "<rootDir>/../ui-core/src/index.tsx",
        "^@sequentech/ui-essentials$": "<rootDir>/../ui-essentials/src/index.tsx",
        // The WASM bindings are an ES module around a .wasm binary, which Jest
        // cannot load. Tests answer for it through src/__mocks__/sequentCore.ts;
        // the real module runs in the Storybook stories and Playwright journeys.
        "^sequent-core$": "<rootDir>/src/__mocks__/sequentCore.ts",
        "\\.(css|png|svg)$": "<rootDir>/src/__mocks__/staticAsset.ts",
    },
    // Coverage runs apply this list to the base revision too, where
    // src/App.test.tsx is still the obsolete Create React App scaffold.
    testMatch: [
        "<rootDir>/src/services/**/*.test.ts",
        "<rootDir>/src/store/**/*.test.ts",
        "<rootDir>/src/providers/**/*.test.tsx",
        "<rootDir>/src/screens/**/*.test.tsx",
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
