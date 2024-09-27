// SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const fse = require("fs-extra")
const path = require("path")

const topDir = __dirname + "/../"
const rootDir = __dirname

fse.emptyDirSync(path.join(topDir, "public", "tinymce"))

fse.copySync(
    path.join(topDir, "node_modules", "tinymce"),
    path.join(rootDir, "public", "tinymce"),
    {
        overwrite: true,
    }
)
