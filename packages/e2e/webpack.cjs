// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
const path = require("path");
const webpack = require("webpack");

module.exports = (env, argv) => {
  const allowed = [
    "admin-portal",
    "voting-portal",
    "ballot-verifier",
    "results-portal",
  ];
  if (!allowed.includes(env.portal)) throw new Error("Unknown E2E portal");
  const directory = path.resolve(__dirname, "..", env.portal);
  process.chdir(directory);
  const config = require(path.join(directory, "webpack.config.cjs"))(env, argv);
  const mode =
    process.env.E2E_BUILD_MODE === "coverage" ? "coverage" : "normal";
  config.context = directory;
  config.cache = {
    type: "filesystem",
    cacheDirectory: path.resolve(__dirname, "../../.e2e/webpack-cache", mode, env.portal),
    buildDependencies: { config: [__filename, path.join(directory, "webpack.config.cjs")] },
  };
  config.output.path = path.resolve(
    __dirname,
    "../../.e2e/web",
    mode,
    env.portal,
  );
  config.resolve.alias = {
    ...config.resolve.alias,
    "sequent-core$": path.resolve(
      __dirname,
      "../../.e2e/wasm/sequent-core/index.js",
    ),
  };
  // Do not embed CI/container secrets through the application's broad env plugin.
  config.plugins = config.plugins.filter(
    (plugin) =>
      !(plugin instanceof webpack.DefinePlugin) &&
      plugin.constructor.name !== "ProgressPlugin",
  );
  config.plugins.push(
    new webpack.DefinePlugin({
      "process.env": JSON.stringify({
        NODE_ENV: "production",
        PUBLIC_URL: "",
        REACT_APP_VERSION: "e2e",
      }),
    }),
  );
  if (mode === "coverage") {
    // webpack's devtool does not enable TypeScript's own source maps. Without
    // this first map, Babel maps Istanbul locations to transpiled JS line numbers
    // while retaining the original .tsx filename, producing false coverage.
    for (const rule of config.module.rules) {
      if (!Array.isArray(rule.use)) continue;
      rule.use = rule.use.map((entry) => {
        const use = typeof entry === "string" ? { loader: entry } : entry;
        if (use.loader === "ts-loader") {
          use.options = { ...use.options, compilerOptions: {
            ...use.options?.compilerOptions, sourceMap: true, inlineSourceMap: false,
          } };
          return use;
        }
        return entry;
      });
    }
    config.module.rules.push({
      test: /\.[jt]sx?$/,
      enforce: "post",
      include: [path.join(directory, "src")],
      exclude: [/\.test\./, /graphql\.ts$/, /setupTests\./],
      use: {
        loader: require.resolve("babel-loader"),
        options: {
          babelrc: false,
          configFile: false,
          sourceMaps: true,
          plugins: [
            [require.resolve("babel-plugin-istanbul"), { cwd: directory }],
          ],
        },
      },
    });
    config.optimization = { ...config.optimization, minimize: false };
  }
  return config;
};
