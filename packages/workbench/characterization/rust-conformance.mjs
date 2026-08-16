// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Equivalence check for the Rust spec crate
// (`packages/workbench/validation-spec` — VALIDATION_LOGIC_DISTILLATION.md
// §5.3 steps 3–4): the typed Rust `f` must agree with what this suite has
// already validated, on two lanes:
//
//   Lane 1 — GROUND TRUTH: every recorded characterization cell. The rule
//     tables' observed columns are real WASM observations and the derived
//     inline views are DOM-validated (dom-validate.md), so Rust must
//     reproduce, per cell: errors, alerts, both gates, tally (vs observed)
//     and all three inline views (vs derived — sound because every
//     recorded cell has match=true, i.e. observed ≡ predicted emissions;
//     asserted here). The classifier's own 32-cell decision table probes
//     Rust `classify` directly, synthetic error states included.
//
//   Lane 2 — RANDOM SWEEP vs spec.mjs: N seeded-random cells over the full
//     policy cross-product × counts × flags, comparing the ENTIRE output
//     structure (emissions, inline views, gate pair, dialog, reachability,
//     tally) against the JS spec. This reaches far beyond the recorded
//     grids (arbitrary bounds, min > max, flag combinations no fixture
//     carries); the JS spec's own authority over the grids comes from the
//     two validation lanes. The seed is fixed for reproducibility
//     (override: --seed=N; cells: --n=N).
//
// Headless; needs cargo (builds `emit-grid` on first run). Writes
// rust-conformance.recorded.json + rust-conformance.md; exits nonzero on
// any disagreement.
//
// Run:  node characterization/rust-conformance.mjs   (from packages/workbench)

import {execFileSync, execSync} from "node:child_process"
import {existsSync, readFileSync, writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {f as jsF, DEFAULTS} from "./spec.mjs"
import {RULE_SPECS} from "./rule-specs.mjs"

const here = path.dirname(fileURLToPath(import.meta.url))
const packagesDir = path.resolve(here, "../..")
const exe = path.join(
    packagesDir,
    "target",
    "release",
    process.platform === "win32" ? "emit-grid.exe" : "emit-grid"
)

const arg = (name, dflt) => {
    const hit = process.argv.find((a) => a.startsWith(`--${name}=`))
    return hit ? Number(hit.split("=")[1]) : dflt
}
const SEED = arg("seed", 20260815)
const N_RANDOM = arg("n", 20000)

// ---------------------------------------------------------------------------
// Build the binary (cheap when fresh) and define the evaluator
// ---------------------------------------------------------------------------
execSync("cargo build --release -p validation-spec --quiet", {
    cwd: packagesDir,
    stdio: ["ignore", "inherit", "inherit"],
})
if (!existsSync(exe)) throw new Error(`emit-grid binary missing at ${exe}`)

const rustEval = (cells) =>
    JSON.parse(
        execFileSync(exe, [], {
            input: JSON.stringify(cells),
            maxBuffer: 1 << 28,
        }).toString()
    )

// ---------------------------------------------------------------------------
// Lane 1 — recorded ground truth
// ---------------------------------------------------------------------------
const RULES = [
    "blank-rule",
    "overvote-rule",
    "undervote-rule",
    "minvote-rule",
    "duprank-rule",
    "prefgaps-rule",
    "invalid-rule",
]
const rec = (name) =>
    JSON.parse(readFileSync(path.join(here, `${name}.recorded.json`), "utf8")).rows

// Canonical comparison: objects with the same values must compare equal
// regardless of key order (serde and JS build keys in different orders);
// array ORDER stays significant (emission order is part of the spec).
const canon = (v) =>
    Array.isArray(v)
        ? v.map(canon)
        : v && typeof v === "object"
          ? Object.fromEntries(Object.keys(v).sort().map((k) => [k, canon(v[k])]))
          : v
const eq = (a, b) => JSON.stringify(canon(a)) === JSON.stringify(canon(b))
const failures = []
let lane1 = 0

for (const rule of RULES) {
    const spec = RULE_SPECS[rule]
    const rows = rec(rule)
    const outs = rustEval(
        rows.map((r) => ({kind: "f", config: spec.specConfig(r), voteState: spec.voteState(r)}))
    )
    rows.forEach((r, i) => {
        lane1++
        const out = outs[i]
        if (!r.match)
            failures.push({lane: 1, rule, cell: r, why: "recorded cell has match=false — derived-inline comparison unsound"})
        const checks = [
            ["errors", eq(out.emissions.errors, r.observed.errors)],
            ["alerts", eq(out.emissions.alerts, r.observed.alerts)],
            ["hard", out.gate.hard === r.observed.hard],
            ["soft", out.gate.soft === r.observed.soft],
            ["tally", out.tally === r.observed.tally],
            ["inline", eq(out.inline, r.derived_inline)],
        ]
        const bad = checks.filter(([, ok]) => !ok).map(([k]) => k)
        if (bad.length)
            failures.push({lane: 1, rule, state: r.state, bad, rust: out, recorded: {observed: r.observed, derived_inline: r.derived_inline}})
    })
}

// The classifier's own decision table — synthetic error states included.
const classifierRows = JSON.parse(
    readFileSync(path.join(here, "classifier-table.recorded.json"), "utf8")
).rows
const classifierOuts = rustEval(
    classifierRows.map((r) => ({
        kind: "classify",
        decline: r.is_decline_to_vote,
        flag: r.is_explicit_invalid,
        hasErrors: r.has_errors,
        selection: r.selection,
    }))
)
classifierRows.forEach((r, i) => {
    lane1++
    if (classifierOuts[i].tally !== r.observed_class)
        failures.push({lane: 1, rule: "classifier-table", cell: r, rust: classifierOuts[i].tally})
})

// ---------------------------------------------------------------------------
// Lane 2 — seeded random sweep vs spec.mjs
// ---------------------------------------------------------------------------
// Mulberry32 — small, deterministic; Math.random is banned by convention
// (reproducibility), the seed is part of the recorded artifact.
function mulberry32(seed) {
    let a = seed >>> 0
    return () => {
        a |= 0
        a = (a + 0x6d2b79f5) | 0
        let t = Math.imul(a ^ (a >>> 15), 1 | a)
        t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
        return ((t ^ (t >>> 14)) >>> 0) / 4294967296
    }
}
const rnd = mulberry32(SEED)
const pick = (xs) => xs[Math.floor(rnd() * xs.length)]
const DOMAINS = {
    invalid: ["allowed", "warn", "warn-invalid-implicit-and-explicit", "not-allowed"],
    blank: ["allowed", "warn", "warn-only-in-review", "not-allowed"],
    over: [
        "allowed",
        "allowed-with-msg",
        "allowed-with-msg-and-alert",
        "not-allowed-with-msg-and-alert",
        "not-allowed-with-msg-and-disable",
    ],
    under: ["allowed", "warn", "warn-only-in-review", "warn-and-alert"],
    dup: ["allowed-warn-and-dialog", "not-allowed-warn-and-dialog"],
    gap: ["allowed-warn-and-dialog", "not-allowed-warn-and-dialog"],
}
const randomCell = () => ({
    kind: "f",
    config: {
        min: Math.floor(rnd() * 5), // 0..4; min > max deliberately possible
        max: Math.floor(rnd() * 5),
        policies: Object.fromEntries(Object.entries(DOMAINS).map(([k, xs]) => [k, pick(xs)])),
    },
    voteState: {
        regulars: Math.floor(rnd() * 5),
        blankMarker: rnd() < 0.3,
        explicitInvalid: rnd() < 0.3,
        decline: rnd() < 0.15,
        duplicateRanks: rnd() < 0.2,
        rankGaps: rnd() < 0.2,
    },
})
const randomCells = Array.from({length: N_RANDOM}, randomCell)
const rustOuts = rustEval(randomCells)
let lane2 = 0
for (let i = 0; i < randomCells.length; i++) {
    lane2++
    const {config, voteState} = randomCells[i]
    // spec.mjs resolves unset policies via DEFAULTS; here all six are set,
    // so the two resolve identically by construction.
    const js = jsF(config, voteState)
    const rust = rustOuts[i]
    const jsShape = {
        emissions: js.emissions,
        inline: js.inline,
        gate: js.gate,
        dialog: js.dialog,
        reachability: js.reachability,
        tally: js.tally,
    }
    if (!eq(rust, jsShape))
        failures.push({lane: 2, i, cell: randomCells[i], rust, js: jsShape})
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------
const total = lane1 + lane2
const ok = failures.length === 0
console.log(
    `lane 1 (recorded ground truth): ${lane1} cells; ` +
        `lane 2 (random vs spec.mjs, seed ${SEED}): ${lane2} cells; ` +
        `disagreements: ${failures.length}`
)
for (const fail of failures.slice(0, 10)) console.log(JSON.stringify(fail))

writeFileSync(
    path.join(here, "rust-conformance.recorded.json"),
    JSON.stringify(
        {seed: SEED, lane1_cells: lane1, lane2_cells: lane2, total, ok, failures},
        null,
        2
    ) + "\n"
)

const md = [
    "<!--",
    " SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>",
    "",
    "SPDX-License-Identifier: AGPL-3.0-only",
    "-->",
    "",
    "# Rust spec conformance",
    "",
    "Generated by `characterization/rust-conformance.mjs`; do not edit by hand.",
    "",
    "**Experiment:** the typed Rust spec",
    "(`packages/workbench/validation-spec`, the §5.3 step-3 artifact) is",
    "evaluated cell-by-cell — a *cell* being one concrete",
    "(contest-configuration × vote-state) combination — and compared on two",
    "lanes. *Lane 1* replays every recorded characterization cell: the seven",
    "rules' grids (each rule's full cross-product of varied policies × vote",
    "states; Rust `f` vs the",
    "wasm-observed errors/alerts/gates/tally and the DOM-validated inline",
    "views) and the classifier's 32-cell decision table (Rust `classify` vs",
    "the recorded class, synthetic error states included). *Lane 2* compares",
    "the ENTIRE output structure against `spec.mjs` on seeded-random cells",
    "over the full policy cross-product × counts 0–4 × flags — including",
    "territory no fixture reaches (arbitrary bounds, min > max, flag",
    "combinations). A disagreement on either lane fails the run.",
    "",
    `| lane | cells | result |`,
    `|---|---|---|`,
    `| 1 — recorded ground truth (7 rule grids + classifier table) | ${lane1} | ${ok ? "all equal" : "DISAGREEMENTS — see rust-conformance.recorded.json"} |`,
    `| 2 — random sweep vs spec.mjs (seed ${SEED}) | ${lane2} | ${ok ? "all equal" : "DISAGREEMENTS — see rust-conformance.recorded.json"} |`,
    "",
    `**${ok ? `${total} cells, Rust ≡ recorded ≡ spec.mjs on every compared component.` : `${failures.length} disagreement(s) — the Rust spec does NOT match; see rust-conformance.recorded.json.`}**`,
    "",
].join("\n")
writeFileSync(path.join(here, "rust-conformance.md"), md)
console.log(`wrote rust-conformance.recorded.json and rust-conformance.md (ok: ${ok})`)
if (!ok) process.exitCode = 1
