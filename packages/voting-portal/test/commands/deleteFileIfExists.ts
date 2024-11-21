// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
const fs = require("fs")
const path = require("path")

exports.command = function (filePath, callback) {
    const fullPath = path.resolve(filePath)
    this.perform((done) => {
        if (fs.existsSync(fullPath)) {
            fs.unlinkSync(fullPath)
        }
        if (typeof callback === "function") {
            callback.call(this)
        }
        done()
    })
    return this
}
