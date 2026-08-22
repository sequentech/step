// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Two lines here decided that the ballot had no tests.
 *
 * `testEnvironment: "node"` means no DOM, so nothing can be mounted. And
 * `testMatch: "*.test.ts"` — without the `x` — means a `.tsx` file is not
 * collected even when somebody writes one. The job named "Run voting-portal
 * tests" was green on eight URL-parsing assertions while `Question`, `Answer`,
 * `AnswersList` and `InvalidErrorsList` had no test of any kind.
 *
 * That mattered the moment those components were about to move into
 * `ui-essentials` to be shared with the Election Architect's ballot preview:
 * restructuring the live voting path is only safe if something can tell you it
 * broke.
 *
 * `jsdom` for everything, because the pure helpers do not mind having a DOM
 * around and one environment is one fewer thing to get wrong.
 */
/** @type {import('jest').Config} */
module.exports = {
    testEnvironment: "jsdom",
    testMatch: ["<rootDir>/src/**/*.test.ts", "<rootDir>/src/**/*.test.tsx"],
    // One copy of these, in the package that will own the ballot. Duplicating a
    // stub is how two stubs come to disagree about what the platform does.
    setupFilesAfterEnv: ["<rootDir>/../ui-essentials/src/testing/setup.ts"],
    transform: {
        "^.+\\.(t|j)sx?$": [
            "@swc/jest",
            {
                jsc: {
                    parser: {syntax: "typescript", tsx: true},
                    target: "es2022",
                    // Without this, swc emits `React.createElement` and every
                    // JSX file fails with "React is not defined" — the app's own
                    // build uses the automatic runtime, so this matches it
                    // rather than requiring a React import per test file.
                    transform: {react: {runtime: "automatic"}},
                },
                module: {type: "commonjs"},
            },
        ],
    },
    moduleNameMapper: {
        // The WASM package is ESM with a `new URL(…, import.meta.url)` in it,
        // which is not loadable under jest's CommonJS transform — and it is the
        // boundary the harness stubs anyway, so it is mapped to the stub for
        // every test.
        "^sequent-core$": "<rootDir>/../ui-essentials/src/testing/sequentCoreStub.ts",

        // The two sibling workspaces, by source rather than by `main`.
        //
        // Both declare `main: dist/index.js`, and neither `dist` exists until
        // somebody runs a webpack build — so without this, every test that
        // touches a shared component fails on "Cannot find module", which reads
        // as a missing dependency rather than a missing build step. It is the
        // same trap `SelectElection.test.tsx` papered over with a virtual mock.
        //
        // Source is also the more honest target: it is what a reader edits, and
        // it means a test cannot pass against a stale bundle.
        "^@sequentech/ui-essentials$": "<rootDir>/../ui-essentials/src/index.tsx",
        "^@sequentech/ui-core$": "<rootDir>/../ui-core/src/index.tsx",

        // Stylesheets a bundler would treat as a side effect. See the stub.
        "\\.(css|less|scss|sass)$": "<rootDir>/../ui-essentials/src/testing/styleStub.ts",

        // `ui-core` resolves its own files through a tsconfig path alias
        // (`"@root/*": ["./src/*"]`), which jest does not read. Without this the
        // failure names a module nobody wrote — `@root/types/LanguageConf`.
        "^@root/(.*)$": "<rootDir>/../ui-core/src/$1",
    },
}
