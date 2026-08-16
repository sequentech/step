// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Effect-dependency analysis — which inputs each effect component depends
// on, does NOT depend on, and under what conditions (the effect-first /
// conditional-independence decomposition; VALIDATION_LOGIC_DISTILLATION.md
// §2 "The seven rules" gives the encoding-side counterpart).
//
// Two lanes, with the epistemics labelled per claim:
//
//   Lane A (spec, exhaustive) — `validation-spec`'s `analyze-deps` bin
//     enumerates the full modelled domain (min ≤ max; ~2M cells, ~29M
//     evaluations) and computes per effect component: support, conditional
//     restrictions (projections: "depends on Y only when Z ∈ S"), and one
//     executable WITNESS per (component, dimension) — a concrete cell pair
//     demonstrating the dependence.
//
//   Lane B (production, headless WASM) — every witness whose cells the
//     harness can represent is re-run through the REAL checker/gates/tally
//     (the same wasm the rule runners record), confirming the dependence
//     exists in production with the same values. Witnesses the lane cannot
//     reach are LABELLED, never silently dropped:
//       browser-pending      — inline/reachability components (filter and
//                              booth-side; a dom-validate extension)
//       preferential state   — needs the IRV ranked recipe (pending)
//       decline              — needs the classifier-direct path (pending)
//       regulars > fixture   — needs a wider contest than the fixtures carry
//       max_votes = 0        — outside every grid; production's config-sanity
//                              checker may intervene (encoding-error scope
//                              boundary)
//
// ACCOUNTING — what this analysis cannot see (the reason lane B exists):
// lane A analyses the SPEC, so in regions no lane has validated, its
// claims describe the transcription, not production. A dependency
// production has that the transcription missed is invisible here by
// construction; the instruments for that residue are the witness
// validation (this file, per tier), the consumer/input censuses, and the
// named scope boundaries (characterization/README.md).
//
// Headless; needs cargo (builds `analyze-deps` on first run) and the
// sequent-core wasm pkg. Writes effect-dependencies.md + .recorded.json;
// exits nonzero if any production-checked witness DISAGREES with the spec.
//
// Run:  node characterization/effect-dependencies.mjs   (from packages/workbench)

import {execFileSync, execSync} from "node:child_process"
import {existsSync, readFileSync, writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {
    loadWasm,
    loadVelvetWasm,
    runChecker,
    runGates,
    tallyClass,
    extractErrors,
} from "./harness.mjs"
import {f as specF} from "./spec.mjs"

const here = path.dirname(fileURLToPath(import.meta.url))
const packagesDir = path.resolve(here, "../..")
const exe = path.join(
    packagesDir,
    "target",
    "release",
    process.platform === "win32" ? "analyze-deps.exe" : "analyze-deps"
)

// ---------------------------------------------------------------------------
// Lane A — run the exhaustive spec analysis
// ---------------------------------------------------------------------------
execSync("cargo build --release -p validation-spec --quiet", {
    cwd: packagesDir,
    stdio: ["ignore", "inherit", "inherit"],
})
if (!existsSync(exe)) throw new Error(`analyze-deps binary missing at ${exe}`)
const analysis = JSON.parse(execFileSync(exe, [], {maxBuffer: 1 << 28}).toString())
console.log(
    `lane A: ${analysis.domain.cells_evaluated} spec evaluations over the ` +
        `modelled domain (${analysis.components.length} components)`
)

// ---------------------------------------------------------------------------
// Lane B — re-run representable witnesses through the real WASM
// ---------------------------------------------------------------------------
await loadWasm()
await loadVelvetWasm()
const snap = JSON.parse(
    readFileSync(
        path.resolve(here, "../app/src/fixtures/snapshots/explicit-blank-invalid.json"),
        "utf8"
    )
)
const eml = Object.values(snap.state.ballotStyles)[0].ballot_eml
// The Referendum contest carries a blank marker, accepts the explicit-invalid
// flag without a marker candidate (recorded: blank-rule.md, explicit_invalid
// state), and has two regular candidates — every plurality witness fits.
const contest = eml.contests.find((c) =>
    c.candidates.some((x) => x.presentation?.is_explicit_blank)
)
const markerId = contest.candidates.find((x) => x.presentation?.is_explicit_blank).id
const regularIds = contest.candidates
    .filter((x) => !x.presentation?.is_explicit_blank)
    .map((x) => x.id)

// A witness cell's dim values arrive as wire strings; parse to spec inputs.
const parseCell = (cell) => ({
    config: {
        min: Number(cell.min_votes),
        max: Number(cell.max_votes),
        policies: {
            invalid: cell.invalid_vote_policy,
            blank: cell.blank_vote_policy,
            over: cell.over_vote_policy,
            under: cell.under_vote_policy,
            dup: cell.duplicated_rank_policy,
            gap: cell.preference_gaps_policy,
        },
    },
    voteState: {
        regulars: Number(cell.regulars),
        blankMarker: cell.blank_marker === "true",
        explicitInvalid: cell.explicit_invalid === "true",
        decline: cell.decline === "true",
        duplicateRanks: cell.duplicate_ranks === "true",
        rankGaps: cell.rank_gaps === "true",
    },
})

const HEADLESS = (name) =>
    name.endsWith("∈ errors") ||
    name.endsWith("∈ alerts") ||
    ["gate.hard", "gate.soft", "dialog", "tally"].includes(name)

const deferReason = (name, cells) => {
    if (!HEADLESS(name)) return "browser-pending (inline/reachability)"
    for (const cell of cells) {
        const c = parseCell(cell)
        if (c.voteState.duplicateRanks || c.voteState.rankGaps)
            return "preferential state (IRV recipe pending)"
        if (c.voteState.decline) return "decline (classifier-direct pending)"
        if (c.voteState.regulars > regularIds.length)
            return `regulars > ${regularIds.length} (no fixture)`
        if (c.config.max === 0)
            return "max_votes = 0 (config-sanity scope boundary)"
    }
    return null
}

function makeEml(config) {
    const clone = structuredClone(eml)
    const c = clone.contests.find((x) => x.id === contest.id)
    c.min_votes = config.min
    c.max_votes = config.max
    c.presentation = {
        ...(c.presentation ?? {}),
        invalid_vote_policy: config.policies.invalid,
        blank_vote_policy: config.policies.blank,
        over_vote_policy: config.policies.over,
        under_vote_policy: config.policies.under,
        duplicated_rank_policy: config.policies.dup,
        preference_gaps_policy: config.policies.gap,
    }
    return clone
}

function makeSelection(vs) {
    const picked = regularIds.slice(0, vs.regulars)
    return {
        contest_id: contest.id,
        is_explicit_invalid: vs.explicitInvalid,
        is_decline_to_vote: false,
        invalid_errors: [],
        invalid_alerts: [],
        choices: contest.candidates.map((c) => ({
            id: c.id,
            selected:
                picked.includes(c.id) || (vs.blankMarker && c.id === markerId) ? 0 : -1,
            write_in_text: null,
        })),
    }
}

const short = (k) => k.split(".").pop()

/** One production observation, projected onto a component name. */
function productionValue(name, obs) {
    if (name.endsWith("∈ errors")) return obs.errors.includes(name.split(" ")[0])
    if (name.endsWith("∈ alerts")) return obs.alerts.includes(name.split(" ")[0])
    if (name === "gate.hard") return obs.hard
    if (name === "gate.soft") return obs.soft
    if (name === "dialog") return obs.hard ? "blocking" : obs.soft ? "dismissible" : "none"
    if (name === "tally") return obs.tally
    throw new Error(`not headless-checkable: ${name}`)
}

/** The spec's value for the same component, from spec.f's output. */
function specValue(name, cellInputs) {
    const e = specF(cellInputs.config, cellInputs.voteState)
    if (name.endsWith("∈ errors")) return e.emissions.errors.map(short).includes(name.split(" ")[0])
    if (name.endsWith("∈ alerts")) return e.emissions.alerts.map(short).includes(name.split(" ")[0])
    if (name === "gate.hard") return e.gate.hard
    if (name === "gate.soft") return e.gate.soft
    if (name === "dialog") return e.dialog
    if (name === "tally") return e.tally
    throw new Error(`not headless-checkable: ${name}`)
}

function observeProduction(cellInputs) {
    const cellEml = makeEml(cellInputs.config)
    const cellContest = cellEml.contests.find((x) => x.id === contest.id)
    const decoded = runChecker(makeSelection(cellInputs.voteState), cellEml)
    const {errors, alerts} = extractErrors(decoded)
    const gates = runGates([cellContest], {[contest.id]: decoded})
    return {
        errors: errors.map(short),
        alerts: alerts.map(short),
        hard: gates.hard,
        soft: gates.soft,
        tally: tallyClass(cellContest, decoded),
    }
}

const checked = []
const deferred = []
const disagreements = []
for (const comp of analysis.components) {
    for (const w of comp.witnesses) {
        const cellA = {...w.cell, [w.varies]: w.from}
        const cellB = {...w.cell, [w.varies]: w.to}
        const reason = deferReason(comp.component, [cellA, cellB])
        if (reason) {
            deferred.push({component: comp.component, varies: w.varies, reason})
            continue
        }
        const row = {component: comp.component, varies: w.varies, cells: [cellA, cellB], ok: true}
        for (const cell of [cellA, cellB]) {
            const inputs = parseCell(cell)
            const prod = productionValue(comp.component, observeProduction(inputs))
            const spec = specValue(comp.component, inputs)
            if (String(prod) !== String(spec)) {
                row.ok = false
                row.divergence = {cell, spec: String(spec), production: String(prod)}
            }
        }
        checked.push(row)
        if (!row.ok) disagreements.push(row)
    }
}
console.log(
    `lane B: ${checked.length} witnesses production-confirmed pairs run, ` +
        `${disagreements.length} disagreements; ${deferred.length} deferred (labelled)`
)

// ---------------------------------------------------------------------------
// Artifacts
// ---------------------------------------------------------------------------
const DIM_SHORT = {
    invalid_vote_policy: "inv",
    blank_vote_policy: "blank",
    over_vote_policy: "over",
    under_vote_policy: "under",
    duplicated_rank_policy: "dupP",
    preference_gaps_policy: "gapP",
    min_votes: "min",
    max_votes: "max",
    regulars: "reg",
    blank_marker: "mkr",
    explicit_invalid: "flag",
    decline: "dec",
    duplicate_ranks: "dupR",
    rank_gaps: "gaps",
}
const dims = analysis.domain.dims.map((d) => d.name)
const POLICY_DIMS = dims.slice(0, 6)

const witnessStatus = new Map() // component → {confirmed, deferredReasons}
for (const row of checked) {
    const s = witnessStatus.get(row.component) ?? {confirmed: 0, reasons: new Set()}
    if (row.ok) s.confirmed++
    witnessStatus.set(row.component, s)
}
for (const d of deferred) {
    const s = witnessStatus.get(d.component) ?? {confirmed: 0, reasons: new Set()}
    s.reasons.add(d.reason)
    witnessStatus.set(d.component, s)
}

const md = []
md.push(
    "<!--",
    " SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>",
    "",
    "SPDX-License-Identifier: AGPL-3.0-only",
    "-->",
    "",
    "# Effect dependencies — support, conditional independence, witnesses",
    "",
    "Generated by `characterization/effect-dependencies.mjs`; do not edit by hand.",
    "",
    "**Experiment.** The effect-first decomposition: for every effect",
    "*component* (each scalar effect of `f` plus the presence of each message",
    "key in errors / alerts / inline.voting / inline.review), which input",
    "dimensions can change it (**support**), which it provably never reads",
    "(everything absent from its support — checked exhaustively), and under",
    "what conditions a dependence is live (**restrictions** — projections of",
    "the sensitive region: \"depends on Y only when Z ∈ S\"). *Lane A*",
    `computes this on the executable spec over its full modelled domain`,
    `(min ≤ max; ${analysis.domain.cells_evaluated.toLocaleString("en-US")} evaluations — the`,
    "cross-product IS materialisable on the spec, unlike on production).",
    "*Lane B* re-runs each dependence's **witness** (a concrete cell pair",
    "proving it) through the real WASM checker/gates/tally where the fixtures",
    "can represent it, and labels every witness it cannot reach.",
    "",
    "**What this analysis cannot see.** Lane A analyses the *spec*: in",
    "regions no validation lane has reached, its claims describe the",
    "transcription, not production — and a dependency production has that the",
    "transcription missed is invisible here by construction. The instruments",
    "for that residue are the witness checks below (per-claim evidence",
    "tiers), the consumer/input censuses and scope boundaries",
    "(`README.md` in this directory), and the browser lane for the",
    "filter/reachability components (pending — see the deferred labels).",
    "",
    `**Lane B result: ${checked.length} witnesses production-confirmed, ` +
        `${disagreements.length} disagreement(s), ${deferred.length} deferred with labels.**`,
    ""
)

md.push(
    "## Support matrix",
    "",
    "One row per non-constant component; ✓ = the dimension can change the",
    "component (some witness proves it), — = provably never (exhaustive over",
    "the modelled domain, spec-side). Column names abbreviate the §2 input",
    "dimensions of VALIDATION_LOGIC_DISTILLATION.md: " +
        dims.map((d) => `*${DIM_SHORT[d]}* = \`${d}\``).join(", ") + ".",
    "",
    `| component | ${dims.map((d) => DIM_SHORT[d]).join(" | ")} |`,
    `|---|${dims.map(() => "---").join("|")}|`
)
for (const c of analysis.components) {
    if (c.constant) continue
    md.push(
        `| ${c.component} | ` +
            dims.map((d) => (c.support.includes(d) ? "✓" : "—")).join(" | ") +
            " |"
    )
}
md.push(
    "",
    "Constant over the whole domain: " +
        analysis.components.filter((c) => c.constant).map((c) => `\`${c.component}\``).join(", ") +
        ".",
    ""
)

md.push(
    "## Conditional independence — policy × policy restrictions",
    "",
    "The config-side conditional statements (the full set of " +
        `${analysis.components.reduce((n, c) => n + c.restrictions.length, 0)} restrictions, ` +
        "including vote-state conditions, is in the recorded JSON). Read each",
    "row as: *the component depends on that dimension only when the",
    "condition holds* (a projection — necessary, not sufficient).",
    "",
    "| component | depends on | only when |",
    "|---|---|---|"
)
for (const c of analysis.components) {
    for (const r of c.restrictions) {
        if (POLICY_DIMS.includes(r.depends_on) && POLICY_DIMS.includes(r.only_when)) {
            md.push(`| ${c.component} | \`${r.depends_on}\` | \`${r.only_when}\` ∈ {${r.in.join(", ")}} |`)
        }
    }
}
md.push("")

md.push(
    "## Witness evidence",
    "",
    "One witness per (component, dimension): a concrete cell pair proving the",
    "dependence, with its lane-B status. *confirmed* = both cells re-run",
    "through the real WASM checker/gates/tally and production agreed with the",
    "spec on both values; deferred labels name what blocks the check (each is",
    "a pending lane, not a verdict).",
    "",
    "Note on the deferred labels: the *preferential state* and *decline*",
    "deferrals are dependencies already exhibited by existing recorded",
    "grids (`duprank-rule.md` / `prefgaps-rule.md` record the gate and tally",
    "sensitivity to the rank dimensions and their policies;",
    "`classifier-table.md` records the decline sensitivity) — the label",
    "means this witness lane has not re-run them itself, not that they are",
    "unvalidated. The *browser-pending* deferrals are genuinely outside",
    "every production lane so far: they are the dom-validate extension this",
    "artifact motivates.",
    "",
    "| component | confirmed | deferred (label: count) |",
    "|---|---|---|"
)
for (const c of analysis.components) {
    if (c.constant) continue
    const s = witnessStatus.get(c.component) ?? {confirmed: 0, reasons: new Set()}
    const defs = deferred.filter((d) => d.component === c.component)
    const byReason = {}
    for (const d of defs) byReason[d.reason] = (byReason[d.reason] ?? 0) + 1
    md.push(
        `| ${c.component} | ${s.confirmed}/${c.witnesses.length} | ` +
            (Object.entries(byReason)
                .map(([r, n]) => `${r}: ${n}`)
                .join("; ") || "—") +
            " |"
    )
}
md.push("")
if (disagreements.length) {
    md.push("## DISAGREEMENTS (spec vs production)", "")
    for (const d of disagreements) {
        md.push(`- ${d.component} varying ${d.varies}: ${JSON.stringify(d.divergence)}`)
    }
    md.push("")
}

writeFileSync(path.join(here, "effect-dependencies.md"), md.join("\n") + "\n")
writeFileSync(
    path.join(here, "effect-dependencies.recorded.json"),
    JSON.stringify(
        {
            domain: analysis.domain,
            components: analysis.components,
            lane_b: {checked, deferred, disagreements},
        },
        null,
        2
    ) + "\n"
)
console.log("wrote effect-dependencies.md and effect-dependencies.recorded.json")
if (disagreements.length) process.exitCode = 1
