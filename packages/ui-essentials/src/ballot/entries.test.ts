// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The portal imports the ballot from the package's main entry and the Election
 * Architect from the ballot entry. A component exported from only one of them is
 * one the other host cannot render: `StartLayout` was missing from the main entry,
 * so the portal's start screen could not use the layout the preview draws.
 */

import {readFileSync} from "node:fs"
import {join} from "node:path"

const valueExports = (path: string): Set<string> => {
    const source = readFileSync(path, "utf8")
    const names = new Set<string>()
    for (const match of source.matchAll(/export\s+\{([^}]*)\}\s+from/g)) {
        for (const entry of match[1].split(",")) {
            const name = entry
                .trim()
                .split(/\s+as\s+/)
                .pop()
                ?.trim()
            if (name) {
                names.add(name)
            }
        }
    }
    return names
}

describe("the package's two entries", () => {
    it("export every ballot component from the main entry too", () => {
        const ballot = valueExports(join(__dirname, "index.ts"))
        const main = valueExports(join(__dirname, "..", "index.tsx"))

        expect([...ballot].filter((name) => !main.has(name))).toEqual([])
    })
})
