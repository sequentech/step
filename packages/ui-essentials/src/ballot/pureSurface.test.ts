// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The ballot bundle compiles `@sequentech/ui-core` in through its `pure` entry, so a
 * name the ballot imports from `ui-core` that `pure.ts` does not export is
 * `undefined` in `ballot.js`. Webpack only warns about it, jest never sees it (jest
 * maps `ui-core` to the full barrel), and the Election Architect's preview throws
 * `isAcclaimedContest is not a function` the first time it draws a contest. That is
 * how `isAcclaimedContest`, `translateFromPresentation`, `translateHtml` and
 * `isEligibleAcclaimedCandidate` went missing after the ovcs merge.
 *
 * Nor may the ballot reach the compiled encoder itself: those calls come from the
 * host through `BallotEngine`.
 */

import {readdirSync, readFileSync} from "node:fs"
import {join} from "node:path"

const PURE = join(__dirname, "..", "..", "..", "ui-core", "src", "pure.ts")

const namesIn = (list: string): Array<string> =>
    list
        .split(",")
        .map((entry) =>
            entry
                .trim()
                .replace(/^type\s+/, "")
                .split(/\s+as\s+/)[0]
                .trim()
        )
        .filter(Boolean)

const pureExports = (): Set<string> => {
    const source = readFileSync(PURE, "utf8")
    const names = new Set<string>()
    for (const match of source.matchAll(/export\s+(?:type\s+)?\{([^}]*)\}\s+from/g)) {
        for (const name of namesIn(match[1])) {
            names.add(name)
        }
    }
    return names
}

const ballotSources = (): Array<string> =>
    readdirSync(__dirname)
        .filter((file) => /\.tsx?$/.test(file) && !/\.(test|stories)\.tsx?$/.test(file))
        .map((file) => join(__dirname, file))

const importsFrom = (path: string, specifier: string): Array<string> => {
    const source = readFileSync(path, "utf8")
    const pattern = new RegExp(
        String.raw`(?:import|export)\s+(?:type\s+)?\{([^}]*)\}\s+from\s+"${specifier}"`,
        "g"
    )
    return [...source.matchAll(pattern)].flatMap((match) => namesIn(match[1]))
}

describe("the ballot's view of ui-core", () => {
    it("imports from @sequentech/ui-core only what its pure entry exports", () => {
        const exported = pureExports()
        const missing = ballotSources().flatMap((path) =>
            importsFrom(path, "@sequentech/ui-core")
                .filter((name) => !exported.has(name))
                .map((name) => `${path.split("/").pop()}: ${name}`)
        )

        expect(missing).toEqual([])
    })

    it("never imports the compiled encoder by value", () => {
        const offenders = ballotSources().filter((path) =>
            /^import\s+(?!type\b)[^;]*from\s+"sequent-core"/m.test(readFileSync(path, "utf8"))
        )

        expect(offenders.map((path) => path.split("/").pop())).toEqual([])
    })
})
