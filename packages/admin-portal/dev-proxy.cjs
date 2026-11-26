// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The development server's same-origin forwards to Harvest, as the deployed
 * web servers forward them (webpack-dev-server `devServer.proxy`).
 */

const UNWRAP_PATH = "/configuration-package-intake/unwrap"

const harvestTarget = (domain) => `http://${domain || "harvest:8400"}`

/** Only this exact path and method reach Harvest's direct package intake. */
const isUnwrapRequest = (pathname, req) =>
    pathname === UNWRAP_PATH && (req?.method ?? "").toUpperCase() === "POST"

const devProxy = (domain) => [
    // Browser trustees reach the board service through Harvest on the
    // portal's own origin. Path, Authorization header and body unchanged.
    {
        context: ["/b4/"],
        target: harvestTarget(domain),
    },
    // An encrypted configuration package's staging document and transfer
    // password, never through Hasura. This server is plain HTTP, so the
    // scheme is overwritten (never passed through from the browser) and the
    // browser's Host is kept: Harvest then accepts only a loopback host.
    {
        context: isUnwrapRequest,
        target: harvestTarget(domain),
        changeOrigin: false,
        xfwd: false,
        // Lowercase: replaces the incoming header instead of adding another.
        headers: {"x-forwarded-proto": "http"},
    },
]

module.exports = {UNWRAP_PATH, devProxy, isUnwrapRequest}
