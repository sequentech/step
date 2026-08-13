// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const path = require("path")

const ESLintPlugin = require("eslint-webpack-plugin")
const {ProgressPlugin} = require("webpack")

/**
 * Two bundles out of one package.
 *
 * `index.js` is the whole library, as it always was: every consumer inside this
 * repository has the same dependency tree, so the barrel's `react-router`,
 * `apexcharts` and `@mui/x-data-grid` cost them nothing.
 *
 * `ballot.js` is the voter's ballot on its own, for a consumer that has none of
 * those — the Election Architect, in the other repository, whose preview draws the
 * real `Question` so that what an election manager approves is what a voter is
 * handed. Importing the barrel would make a configuration wizard install a charting
 * library and a data grid to draw a ballot, and worse: the barrel reaches
 * `@sequentech/ui-core`, whose own barrel re-exports `services/wasm`, whose first
 * statement loads the compiled encoder. The wizard already has that encoder, built
 * from the same Rust with a different feature set, so it would end up with two.
 *
 * Hence two differences in the ballot config, and only two: `@sequentech/ui-core`
 * is *bundled* rather than external, aliased to its `pure` entry point so nothing
 * compiled comes with it; and the four calls that genuinely need the encoder are not
 * here at all — a host passes them in through `BallotEngine`.
 */
/**
 * What the ballot bundle does not carry.
 *
 * **By prefix, not by exact name, and that distinction cost a day.** Listing
 * `"@mui/material"` externalises only the barrel: a deep import — `@mui/material/Box`,
 * and everything MUI itself reaches, `@mui/system`, `@mui/styled-engine`,
 * `@mui/utils` — did not match, so webpack *bundled MUI's CommonJS build into this
 * file*. Two consequences, one invisible and one worse than invisible:
 *
 *  - The host ends up with **two copies of MUI**, one in its own graph and one inside
 *    this bundle, with separate theme contexts.
 *  - Those bundled CommonJS modules reach their dependencies through
 *    `_interopRequireDefault`, which assumes webpack resolved them. Hand the bundle
 *    to another bundler and the assumption fails: `(0 , a.default) is not a function`,
 *    thrown from minified MUI while `Box` renders. That is why jest here cannot load
 *    the real ballot, and why the wizard's **Storybook** rendered nothing at all —
 *    every story module failed to register, so all 33 pictures in the delivery guide
 *    came back as "rendered nothing", naming neither MUI nor this file.
 *
 * A prefix regex leaves every one of those paths to the host, which is what a peer
 * dependency means. The bundle then holds this repository's components and nothing
 * else — the point of it.
 */
const BALLOT_EXTERNALS = [
    // Exact, because these have no subpaths worth externalising differently.
    {
        "react": "react",
        "react-dom": "react-dom",
        "react-i18next": "react-i18next",
    },
    // By prefix, so deep imports go to the host too.
    /^@mui\//,
    /^@emotion\//,
    /^@fortawesome\//,
    /^react-dom\//,
    /^react\//,
]

module.exports = function (env, argv) {
    const ballotOnly = process.env.BALLOT_ENTRY === "1"
    return {
        mode: argv.mode,
        entry: ballotOnly
            ? path.resolve(__dirname, "src/ballot/index.ts")
            : path.resolve(__dirname, "src/index.tsx"),
        output: {
            filename: ballotOnly ? "ballot.js" : "index.js",
            library: {
                type: "module", // <-- ES module, no UMD bootstrap
            },
            path: path.resolve(__dirname, "dist"),
            // Not on the ballot pass: it runs second and would delete `index.js`.
            clean: !ballotOnly,
        },
        experiments: {outputModule: true},

        /**
         * How the ballot bundle reaches the packages it does not carry.
         *
         * Only on this pass, and it is load-bearing. The default for an ES module
         * library is `externalsType: "module"`, which imports every external as a
         * *namespace* and then reaches any default export through webpack's own
         * `__webpack_require__.n` interop helper. That helper assumes webpack's module
         * objects: given a namespace that says `__esModule` but carries no `default`,
         * it hands back `undefined`, and the first render dies with
         *
         *     TypeError: (0 , a.default) is not a function
         *
         * twelve frames inside somebody else's minified output. Inside webpack that
         * never happens, so the bundle looked fine in the wizard while being unusable
         * anywhere else: it is why jest in beyond cannot load the real ballot, and it
         * silently emptied the wizard's **Storybook**, where Vite re-bundles this file
         * — every story module failed to register and all 33 pictures in the delivery
         * guide came out as "rendered nothing", with no error naming this bundle.
         *
         * `module-import` emits the import *form the source wrote*: a default import
         * stays `import styled from "@emotion/styled"`, so there is no helper and no
         * assumption to break. Note that `commonjs2` is not the answer either — it
         * moves the exports to runtime, and Rollup then cannot see them
         * (`"getImageUrl" is not exported by ballot.js`).
         */
        ...(ballotOnly ? {externalsType: "module-import"} : {}),
        devtool: "source-map",
        module: {
            rules: [
                // `ui-core`'s own source, transpiled but not typechecked here.
                //
                // The ballot bundle compiles `ui-core/src/pure.ts` into itself rather
                // than leaving `@sequentech/ui-core` external, because that barrel
                // loads the compiled encoder and a consumer with its own build would
                // then have two. But `ts-loader` resolves modules through *this*
                // package's tsconfig, and `ui-core` uses `@root/*` for its own `src`
                // — so typechecking its files from here fails on
                // `@root/types/LanguageConf` no matter what webpack's alias says. A
                // webpack alias cannot fix it; only a tsconfig can, and one tsconfig
                // cannot give `@root` two meanings.
                //
                // So this rule transpiles them and nothing more. `ui-core` typechecks
                // its own source (`yarn --cwd ../ui-core tsc --noEmit`, 0 errors), and
                // the ballot's *use* of what it imports is still checked, because
                // those imports resolve to real declarations.
                ...(ballotOnly
                    ? [
                          {
                              test: /\.(js|ts)x?$/,
                              include: path.resolve(__dirname, "../ui-core/src"),
                              // The presets are spelled out because `babel-loader`
                              // runs alone here: in the pair below, `ts-loader` strips
                              // the types before babel sees them, so the shared babel
                              // config never needed a TypeScript preset — and without
                              // one, babel reads `export type {…}` as Flow and says so.
                              use: [
                                  {
                                      loader: "babel-loader",
                                      options: {
                                          presets: [
                                              ["@babel/preset-env", {targets: {esmodules: true}}],
                                              ["@babel/preset-react", {runtime: "automatic"}],
                                              "@babel/preset-typescript",
                                          ],
                                      },
                                  },
                              ],
                          },
                      ]
                    : []),
                {
                    test: /\.(js|ts)x?$/,
                    exclude: ballotOnly
                        ? [/node_modules/, path.resolve(__dirname, "../ui-core/src")]
                        : /node_modules/,
                    use: ["babel-loader", "ts-loader"],
                },
                {
                    type: "asset",
                    test: /\.(png|jpe?g|gif|ico|svg)$/i,
                },
            ],
        },
        externals: ballotOnly
            ? BALLOT_EXTERNALS
            : {
                  "react": "react",
                  "react-dom": "react-dom",
                  "react-router": "react-router",
                  "@mui/material": "@mui/material",
                  "mui-image": "mui-image",
                  "@emotion/react": "@emotion/react",
                  "@emotion/styled": "@emotion/styled",
                  "react-i18next": "react-i18next",
                  "react-apexcharts": "react-apexcharts",
                  "@mui/x-data-grid": "@mui/x-data-grid",
                  "@sequentech/ui-core": "@sequentech/ui-core",
              },
        resolve: {
            alias: {
                // `@root` means two different things in two packages, and the ballot
                // bundle compiles both. `ui-core` uses it for its own `src`
                // (`"@root/*": ["./src/*"]` in its tsconfig); nothing in
                // `ui-essentials/src` uses it at all — checked — so on the ballot pass
                // it points at `ui-core`, which is the only package that needs it.
                // Without this, `ui-core/src/services/i18n.ts` resolves
                // `@root/types/LanguageConf` into *this* package and fails to compile.
                "@root": ballotOnly
                    ? path.resolve(__dirname, "../ui-core/src")
                    : path.resolve(__dirname, "src"),
                // The WebAssembly-free door into `ui-core`. Only for the ballot
                // bundle: the library bundle keeps `ui-core` external, so the
                // repository's own consumers are unaffected.
                ...(ballotOnly
                    ? {
                          "@sequentech/ui-core": path.resolve(__dirname, "../ui-core/src/pure.ts"),
                      }
                    : {}),
            },
            extensions: [".js", ".jsx", ".ts", ".tsx"],
        },
        plugins: [
            new ProgressPlugin(),
            new ESLintPlugin({
                extensions: [".js", ".jsx", ".ts", ".tsx"],
            }),
        ],
    }
}
