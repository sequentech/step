// SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const fse = require("fs-extra")
const path = require("path")

const topDir = __dirname + "/../"
const rootDir = __dirname

// Copy TinyMCE files
fse.emptyDirSync(path.join(topDir, "public", "tinymce"))

fse.copySync(
    path.join(topDir, "node_modules", "tinymce"),
    path.join(rootDir, "public", "tinymce"),
    {
        overwrite: true,
    }
)

// Copy SQL.js WASM files directly to public root
const sqlJsSourceDir = path.join(topDir, "node_modules", "sql.js", "dist")
const publicDir = path.join(rootDir, "public")

// Copy WASM file
if (fse.existsSync(path.join(sqlJsSourceDir, "sql-wasm.wasm"))) {
    fse.copySync(
        path.join(sqlJsSourceDir, "sql-wasm.wasm"),
        path.join(publicDir, "sql-wasm.wasm"),
        {
            overwrite: true,
        }
    )
    console.log("✓ Copied sql-wasm.wasm to public/")
}

// Copy JS file
if (fse.existsSync(path.join(sqlJsSourceDir, "sql-wasm.js"))) {
    fse.copySync(path.join(sqlJsSourceDir, "sql-wasm.js"), path.join(publicDir, "sql-wasm.js"), {
        overwrite: true,
    })
    console.log("✓ Copied sql-wasm.js to public/")
}
