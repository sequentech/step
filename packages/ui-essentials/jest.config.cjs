// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * `jsdom`, so a component can be interacted with rather than only stringified.
 *
 * This was `testEnvironment: "node"`, and every one of the four tests here used
 * `renderToStaticMarkup` with `expect(markup).toContain(...)`. That can assert
 * what a component renders once; it cannot click, focus, type or observe a state
 * change — which is most of what a ballot does. `@testing-library/react` was
 * already a devDependency and unused, for want of a DOM.
 *
 * It matters now because the ballot — `Question`, `Answer`, `AnswersList`,
 * `InvalidErrorsList` — is moving into this package to be shared with the Election
 * Architect's preview, and `Candidate`, the row those components draw, has no test
 * at all. jsdom is a superset of what the existing four need, so they keep passing
 * unchanged.
 */
module.exports = {
    testEnvironment: "jsdom",
    setupFilesAfterEnv: ["<rootDir>/src/testing/setup.ts"],
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
