// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// reproduce-verify — thin orchestrator over the three end-to-end pipeline
// runners that confirm the findings in `docs/REPRODUCE.md`. It runs each,
// checks its exit code AND its recorded confirmed-flag, and reports one
// aggregate verdict (nonzero exit on any failure).
//
// Why an orchestrator, not its own verifier: each REPRODUCE.md finding is
// already confirmed booth-to-tally (booth → encrypt → cast → decrypt → decode
// → tally) by a dedicated pipeline runner. This script used to RE-verify them
// a second way — configuration through the Policy-overrides panel (the
// reviewer path) instead of `window.__store.dispatch`. That panel path is now
// exercised across EVERY cell by `dom-validate.mjs` (which configures through
// the panel on purpose), so a bespoke panel re-verification here was
// redundant. What stays uniquely valuable is the end-to-end crypto
// confirmation the three runners do — so this simply runs them and aggregates.
//
// The runners each reset + reload the fixture via the global
// `__resetWorkbench`, so they must run SEQUENTIALLY (parallel runs would
// clobber each other's workbench state). Requires the dev server on :5173.

import {spawnSync} from "node:child_process"
import {readFileSync, writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"

const here = path.dirname(fileURLToPath(import.meta.url))

// Each runner writes a recorded JSON with a confirmed-flag; the key differs
// (a single cell vs a set), so each names how to read its verdict.
const RUNNERS = [
    {
        finding: "S1 — over-vote silently discarded",
        script: "overvote-e2e-pipeline.mjs",
        recorded: "overvote-e2e-pipeline.recorded.json",
        confirmed: (d) => d.confirmed_end_to_end === true,
    },
    {
        finding: "S2 — deliberate blank below min_votes silently discarded",
        script: "minvote-e2e-pipeline.mjs",
        recorded: "minvote-e2e-pipeline.recorded.json",
        confirmed: (d) => d.all_confirmed === true,
    },
    {
        finding: "S5 — null vote preserves the candidate selection in the ciphertext",
        script: "invalid-latent-choices-e2e.mjs",
        recorded: "invalid-latent-choices-e2e.recorded.json",
        confirmed: (d) => d.confirmed_end_to_end === true,
    },
]

const results = []
for (const r of RUNNERS) {
    console.log(`\n=== ${r.finding} ===`)
    console.log(`(running ${r.script})`)
    const run = spawnSync(process.execPath, [path.join(here, r.script)], {stdio: "inherit"})
    const exitOk = run.status === 0
    let recordedConfirmed = null
    try {
        recordedConfirmed = r.confirmed(JSON.parse(readFileSync(path.join(here, r.recorded), "utf8")))
    } catch {
        recordedConfirmed = null // recorded file missing or unparsable
    }
    const pass = exitOk && recordedConfirmed === true
    results.push({finding: r.finding, script: r.script, exit: run.status, recordedConfirmed, pass})
    console.log(`→ ${r.script}: exit=${run.status}, recorded confirmed=${recordedConfirmed}, PASS=${pass}`)
}

const allPass = results.every((r) => r.pass)
console.log(`\nall REPRODUCE.md findings confirmed end-to-end: ${allPass}`)
for (const r of results) if (!r.pass) console.log(`  FAILED: ${r.finding} (${r.script})`)

writeFileSync(
    path.join(here, "reproduce-verify.recorded.json"),
    JSON.stringify({role: "orchestrator over the *-e2e pipeline runners", runners: results, all_pass: allPass}, null, 2) + "\n"
)
console.log("wrote reproduce-verify.recorded.json")
if (!allPass) process.exitCode = 1
