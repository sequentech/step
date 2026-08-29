// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Batch evaluator for the Rust spec crate (`../validation-spec`) — the
// shared entry point every runner uses to ask the spec what a cell should
// produce (docs/EVIDENCE_RESTRUCTURE.md, step 0).
//
// The crate's `emit-grid` binary reads a JSON array of cells from stdin and
// writes the corresponding array of outputs to stdout, so evaluation is one
// process per batch rather than one per cell. Callers therefore collect
// their cells first and evaluate once; at sweep scale (10^5 cells) the
// per-process cost is irrelevant and the per-cell cost would not be.
//
// Batches are chunked because stdout has to be buffered whole: an Effects
// record serializes to a few hundred bytes, so a six-figure sweep would
// otherwise push tens of megabytes through a single pipe read.
//
// The build is lazy and happens once per process — cheap when the binary is
// already fresh, and callers should not have to sequence it themselves.
// Needs `cargo`; no wasm, no browser.

import {execFileSync, execSync} from "node:child_process"
import {existsSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"

const here = path.dirname(fileURLToPath(import.meta.url))
const packagesDir = path.resolve(here, "../..")
const exe = path.join(
    packagesDir,
    "target",
    "release",
    process.platform === "win32" ? "emit-grid.exe" : "emit-grid"
)

const CHUNK = 20000

let built = false

/** Build `emit-grid` once per process; returns its path. */
export function buildEmitGrid() {
    if (built) return exe
    execSync("cargo build --release -p validation-spec --quiet", {
        cwd: packagesDir,
        stdio: ["ignore", "inherit", "inherit"],
    })
    if (!existsSync(exe)) throw new Error(`emit-grid binary missing at ${exe}`)
    built = true
    return exe
}

function run(cells) {
    if (cells.length === 0) return []
    buildEmitGrid()
    const out = []
    for (let i = 0; i < cells.length; i += CHUNK) {
        const batch = JSON.parse(
            execFileSync(exe, [], {
                input: JSON.stringify(cells.slice(i, i + CHUNK)),
                maxBuffer: 1 << 28,
            }).toString()
        )
        for (const o of batch) out.push(o)
    }
    return out
}

/**
 * Evaluate the mapping on a batch of cells.
 * @param {{config: object, voteState: object}[]} cells
 * @returns {object[]} one `Effects` record per cell, in order
 */
export function specF(cells) {
    return run(
        cells.map((c) => ({kind: "f", config: c.config, voteState: c.voteState}))
    )
}

/**
 * The RATIONALIZED implementation's mapping (`f_fixed`, the query-provider) —
 * the "after" leg of the diff report. Same cell shape as {@link specF}.
 * @param {{config: object, voteState: object}[]} cells
 * @returns {object[]} one `Effects` record per cell, in order
 */
export function specFixed(cells) {
    return run(
        cells.map((c) => ({kind: "fixed", config: c.config, voteState: c.voteState}))
    )
}

/**
 * Probe the classifier directly on a batch of hand-shaped decoded ballots —
 * the inputs decode cannot reach (decline; see `classifier-table.mjs`).
 * @param {{decline: boolean, flag: boolean, hasErrors: boolean, selection: string}[]} cells
 * @returns {string[]} one BallotClass per cell, in order
 */
export function specClassify(cells) {
    return run(
        cells.map((c) => ({
            kind: "classify",
            decline: c.decline,
            flag: c.flag,
            hasErrors: c.hasErrors,
            selection: c.selection,
        }))
    ).map((o) => o.tally)
}

/**
 * BallotValidator's cross-contest gate OR for a batch of ballots. Each ballot
 * is an array of per-contest {config, voteState}.
 * @param {{config: object, voteState: object}[][]} ballots
 * @returns {{hard: boolean, soft: boolean}[]} one per ballot, in order
 */
export function specBallot(ballots) {
    return run(
        ballots.map((contests) => ({
            kind: "ballot",
            contests: contests.map((c) => ({config: c.config, voteState: c.voteState})),
        }))
    )
}
