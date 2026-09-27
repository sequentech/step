// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * `ballot.js` leaves every `@mui/*`, `@emotion/*` and `@fortawesome/*` path, and
 * React, to the host (see `BALLOT_EXTERNALS` in `webpack.config.cjs`). A package the
 * bundle imports but this package does not declare gives the host no install hint:
 * inside this repository hoisting hides it, and outside it — the Election Architect
 * — `ballot.js` fails to resolve the module. `IvrCall` and `SupportMaterialsLayout`
 * imported `@mui/icons-material` that way.
 *
 * So walk what the ballot entry reaches through relative imports and check every
 * external it names is a dependency or a peer dependency. What webpack bundles is
 * not the host's business, so only `BALLOT_EXTERNALS` matches count.
 */

import {existsSync, readFileSync, statSync} from "node:fs"
import {createRequire} from "node:module"
import {dirname, join, resolve} from "node:path"

const ROOT = join(__dirname, "..", "..")
const ENTRY = join(__dirname, "index.ts")

type External = Record<string, string> | RegExp
const {BALLOT_EXTERNALS} = createRequire(__filename)(join(ROOT, "webpack.config.cjs")) as {
    BALLOT_EXTERNALS: Array<External>
}

/** Whether webpack leaves this import to the host rather than bundling it. */
const isExternal = (specifier: string): boolean =>
    BALLOT_EXTERNALS.some((external) =>
        external instanceof RegExp ? external.test(specifier) : specifier in external
    )

const resolveRelative = (from: string, specifier: string): string | undefined => {
    const base = resolve(dirname(from), specifier)
    for (const candidate of [
        base,
        `${base}.ts`,
        `${base}.tsx`,
        join(base, "index.ts"),
        join(base, "index.tsx"),
    ]) {
        if (existsSync(candidate) && statSync(candidate).isFile()) {
            return candidate
        }
    }
    return undefined
}

const packageName = (specifier: string): string =>
    specifier.startsWith("@") ? specifier.split("/").slice(0, 2).join("/") : specifier.split("/")[0]

/** Packages the ballot bundle leaves to its host, and which file first imported each. */
const ballotPackages = (): Map<string, string> => {
    const seen = new Set<string>()
    const packages = new Map<string, string>()
    const queue = [ENTRY]
    while (queue.length) {
        const file = queue.pop() as string
        if (seen.has(file) || !/\.tsx?$/.test(file)) {
            continue
        }
        seen.add(file)
        const source = readFileSync(file, "utf8")
        const pattern =
            /^\s*(?:import|export)\s+(type\s+)?(?:[^"';]*?\s+from\s+)?["']([^"']+)["']/gm
        for (const [, typeOnly, specifier] of source.matchAll(pattern)) {
            if (typeOnly) {
                continue
            }
            if (specifier.startsWith(".")) {
                const target = resolveRelative(file, specifier)
                if (target) {
                    queue.push(target)
                }
                continue
            }
            const name = packageName(specifier)
            if (isExternal(specifier) && !packages.has(name)) {
                packages.set(name, file.slice(ROOT.length + 1))
            }
        }
    }
    return packages
}

describe("what ballot.js leaves to its host", () => {
    it("is declared as a dependency or a peer dependency", () => {
        const manifest = JSON.parse(readFileSync(join(ROOT, "package.json"), "utf8"))
        const declared = new Set([
            ...Object.keys(manifest.dependencies ?? {}),
            ...Object.keys(manifest.peerDependencies ?? {}),
        ])

        const undeclared = [...ballotPackages()]
            .filter(([name]) => !declared.has(name))
            .map(([name, file]) => `${name} (${file})`)

        expect(undeclared).toEqual([])
    })
})
