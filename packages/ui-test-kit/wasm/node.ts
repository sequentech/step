// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {createRequire} from "node:module"
import {readFileSync} from "node:fs"
import {pathToFileURL} from "node:url"

/** Load the same checked-in WASM archive as the portal, outside its application wrappers. */
export async function loadCore() {
    const require = createRequire(__filename)
    const entry = require.resolve("sequent-core")
    const core: typeof import("sequent-core") = await import(pathToFileURL(entry).href)
    core.initSync({module: readFileSync(entry.replace(/\.js$/, "_bg.wasm"))})
    return core
}
