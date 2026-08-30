// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const path = require("path")

const locStudioRoot = path.resolve(__dirname, "../../../beyond/packages/voting-portal-loc-studio")
const locStudioWebpack = require(path.join(locStudioRoot, "webpack.config.cjs"))

module.exports = function (env, argv) {
    const config = locStudioWebpack(env, argv)
    return {
        ...config,
        entry: path.resolve(locStudioRoot, "src/standalone/index.tsx"),
        output: {
            ...config.output,
            path: path.resolve(locStudioRoot, "dist"),
        },
    }
}
