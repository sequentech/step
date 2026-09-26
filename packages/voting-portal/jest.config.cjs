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
    // The ballot now lives in ui-essentials, shared with the Election
    // Architect's preview; its test setup (jest-dom, matchMedia and
    // ResizeObserver stubs) is shared too, so the two packages' stubs cannot
    // disagree about what the platform does.
    setupFilesAfterEnv: [
        "<rootDir>/src/setupTests.ts",
        "<rootDir>/../ui-essentials/src/testing/setup.ts",
    ],
    // The shared UI package entries point at dist/ bundles, which a clean
    // `yarn install` doesn't produce, so both are mapped to their sources.
    moduleNameMapper: {
        // The WASM package is ESM with a `new URL(…, import.meta.url)` in it,
        // which jest's CommonJS transform cannot load. It is the boundary the
        // ballot's engine is injected at, so every test gets the stub.
        "^sequent-core$": "<rootDir>/../ui-essentials/src/testing/sequentCoreStub.ts",
        // The ballot components moved to ui-essentials reach ui-core helpers
        // (candidate categorisation, checkable options, ...) beyond the subset
        // in src/__mocks__/uiCoreTestEntry.ts, so ui-core maps to its full
        // source; the WASM underneath it is stubbed above.
        "^@sequentech/ui-core$": "<rootDir>/../ui-core/src/index.tsx",
        "^@sequentech/ui-essentials$": "<rootDir>/../ui-essentials/src/index.tsx",
        // ui-core resolves its own files through a tsconfig path alias
        // (`"@root/*": ["./src/*"]`), which jest does not read.
        "^@root/(.*)$": "<rootDir>/../ui-core/src/$1",
        "\\.(css|less|scss|sass)$": "<rootDir>/../ui-essentials/src/testing/styleStub.ts",
        "\\.(png|svg)$": "<rootDir>/src/__mocks__/staticAsset.ts",
    },
    // Unimported runtime files remain in the denominator. Only Jest support
    // and tests are excluded; production fixtures and entry points stay.
    collectCoverageFrom: [
        "src/**/*.{ts,tsx}",
        "!src/**/*.d.ts",
        "!src/**/*.test.{ts,tsx}",
        "!src/__mocks__/**",
        "!src/setupTests.ts",
        "!src/setupJestGlobals.ts",
        "!src/**/*.stories.{ts,tsx}",
        "!src/**/__stories__/**",
    ],
    coverageProvider: "babel",
    coverageDirectory: "coverage",
    coverageReporters: ["text", "html", "lcov", "json", "json-summary"],
    // CI compares every measured metric against the PR base.
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
}
