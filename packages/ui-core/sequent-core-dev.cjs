// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Resolves `sequent-core` to the development package that `scripts/dev/step-dev
// wasm` publishes, and otherwise leaves the installed tgz in place. The package
// lives outside node_modules because webpack snapshots those paths as immutable
// and would not notice a republished build.
const fs = require("fs")
const path = require("path")

const PACKAGES = path.resolve(__dirname, "..")
const DEV_PACKAGE = ["rust-local-target", "sequent-core-wasm", "dist"]
const WEBPACK_DEVELOPMENT = "development"
const FAILED = "failed"

const devPackage = (packagesDir = PACKAGES) => path.join(packagesDir, ...DEV_PACKAGE)

/** The status `step-dev wasm` recorded, or undefined when nothing was published. */
function sequentCoreDevStatus(packagesDir) {
    try {
        return JSON.parse(
            fs.readFileSync(path.join(devPackage(packagesDir), "status.json"), "utf8")
        )
    } catch {
        return undefined
    }
}

function devEntry(packagesDir) {
    const entry = path.join(devPackage(packagesDir), "index.js")
    if (!fs.existsSync(entry)) return undefined
    const status = sequentCoreDevStatus(packagesDir)
    const message = `sequent-core: loading development build ${status?.published?.build} from ${entry}`
    if (status?.state === FAILED) {
        console.warn(`${message}; its last rebuild failed, run scripts/dev/step-dev wasm`)
    } else {
        console.info(message)
    }
    return entry
}

/** webpack `resolve.alias` entries; only development mode loads the development package. */
function sequentCoreWebpackAlias(mode, packagesDir) {
    const entry = mode === WEBPACK_DEVELOPMENT ? devEntry(packagesDir) : undefined
    return entry ? {"sequent-core$": entry} : {}
}

/** Vite `resolve.alias` entries for the development package, when there is one. */
function sequentCoreViteAlias(packagesDir) {
    const entry = devEntry(packagesDir)
    return entry ? [{find: /^sequent-core$/, replacement: entry}] : []
}

module.exports = {sequentCoreDevStatus, sequentCoreWebpackAlias, sequentCoreViteAlias}
