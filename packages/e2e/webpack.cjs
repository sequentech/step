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
