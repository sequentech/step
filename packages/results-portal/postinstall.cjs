// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const fse = require("fs-extra")
const path = require("path")

const topDir = path.join(__dirname, "..")
const rootDir = __dirname
const sqlJsSourceDir = path.join(topDir, "node_modules", "sql.js", "dist")
const publicDir = path.join(rootDir, "public")

for (const file of ["sql-wasm.wasm", "sql-wasm.js"]) {
    const source = path.join(sqlJsSourceDir, file)
    if (fse.existsSync(source)) {
        fse.copySync(source, path.join(publicDir, file), {overwrite: true})
        console.log(`Copied ${file} to results-portal/public/`)
    }
}
