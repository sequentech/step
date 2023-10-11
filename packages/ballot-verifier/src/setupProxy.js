// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
module.exports = function (app) {
    app.use(function (req, res, next) {
        res.setHeader("Cross-Origin-Embedder-Policy", "require-corp")
        res.setHeader("Cross-Origin-Opener-Policy'", "same-origin")
        next()
    })
}
