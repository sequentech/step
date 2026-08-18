// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Effect-dependency analysis — which inputs each effect component depends
// on, does NOT depend on, and under what conditions (the effect-first /
// conditional-independence decomposition; VALIDATION_LOGIC_DISTILLATION.md
// §2 "The seven rules" gives the encoding-side counterpart).
//
// A pure ANALYSIS of the spec — it never touches production. Fidelity
// (production ≡ this spec) is established elsewhere, by `headless-sweep.mjs`;
// this consumes that and asks what the certified mapping depends on.
//
// `validation-spec`'s `analyze-deps` bin enumerates the full modelled domain
// (min ≤ max; ~2M cells, ~29M evaluations) and computes per effect
// component: its support, its conditional restrictions (projections:
// "depends on Y only when Z ∈ S"), and one executable WITNESS per
// (component, dimension) — a concrete cell pair demonstrating the
// dependence.
//
// Each witness is then checked for MEMBERSHIP OF THE SWEPT DOMAIN rather
// than re-run through production. The sweep compares production against this
// same spec on every cell of that domain, so a witness inside it is already
// production-confirmed; re-running it proved nothing and made the evidence
// look independent when it was not. Witnesses outside are LABELLED, never
// silently dropped:
//       browser-pending      — inline/reachability components (filter and
//                              booth-side; browser-witnesses.mjs covers these)
//       decline              — needs the classifier-direct path (pending)
//       regulars > fixture   — needs a wider contest than the fixtures carry
//       max_votes = 0        — outside every grid; production's config-sanity
//                              checker may intervene (encoding-error scope
//                              boundary)
//
// ACCOUNTING — what this analysis cannot see: it analyses the SPEC, so in
// regions the sweep has not certified, its claims describe the
// transcription, not production. A dependency production has that the
// transcription missed is invisible here by construction; the instruments
// for that residue are the sweep's coverage, the browser stages, the
// consumer/input censuses, and the named scope boundaries
// (characterization/README.md).
//
// Headless; needs cargo (builds `analyze-deps` on first run) — no wasm and
// no browser, since nothing here observes production. Writes
// effect-dependencies.md + .recorded.json;
// exits nonzero if any witness cell falls outside the swept domain.
//
// Run:  node characterization/effect-dependencies.mjs   (from packages/workbench)

import {execFileSync, execSync} from "node:child_process"
import {existsSync, readFileSync, writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {representable, shortKey} from "./cell.mjs"

const here = path.dirname(fileURLToPath(import.meta.url))
const packagesDir = path.resolve(here, "../..")
const exe = path.join(
    packagesDir,
    "target",
    "release",
    process.platform === "win32" ? "analyze-deps.exe" : "analyze-deps"
)

// ---------------------------------------------------------------------------
// The exhaustive spec analysis
// ---------------------------------------------------------------------------
execSync("cargo build --release -p validation-spec --quiet", {
    cwd: packagesDir,
    stdio: ["ignore", "inherit", "inherit"],
})
if (!existsSync(exe)) throw new Error(`analyze-deps binary missing at ${exe}`)
const analysis = JSON.parse(execFileSync(exe, [], {maxBuffer: 1 << 28}).toString())
console.log(
    `spec analysis: ${analysis.domain.cells_evaluated} evaluations over the ` +
        `modelled domain (${analysis.components.length} components)`
)

// ---------------------------------------------------------------------------
// Production coverage — inherited from the sweep, not re-observed
//
// This stage used to re-run each representable witness pair through the real
// WASM. That is now redundant: `headless-sweep.mjs` compares production
// against this same Rust spec on EVERY cell of the representable headless
// domain, so a witness inside that domain is already production-confirmed
// and re-running it proves nothing new. Worse, keeping it made the evidence
// story look like it carried an independent check that it did not.
//
// What replaces it is a CHECK, not a claim: every witness cell that the old
// lane would have run is tested for membership of the swept domain, and any
// cell outside it is reported rather than quietly assumed covered.
// ---------------------------------------------------------------------------

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
        const reason = representable(parseCell(cell))
        if (reason) return reason
    }
    return null
}

/** Is this cell inside the domain `headless-sweep.mjs` enumerates? Mirrors
 *  that runner's BOUNDS and STATES; `representable()` carries the rest
 *  (fixture limits, the ranked triples, the codec's rank/max_votes rule). */
function inSweptDomain(inputs) {
    if (representable(inputs)) return false
    const {config: c, voteState: vs} = inputs
    if (!(c.min >= 0 && c.min <= 3 && c.max >= 1 && c.max <= 3 && c.min <= c.max))
        return false
    if (vs.duplicateRanks || vs.rankGaps) return !vs.blankMarker
    return vs.regulars <= 2
}

const covered = []
const deferred = []
const outsideSweptDomain = []
for (const comp of analysis.components) {
    for (const w of comp.witnesses) {
        const cellA = {...w.cell, [w.varies]: w.from}
        const cellB = {...w.cell, [w.varies]: w.to}
        const reason = deferReason(comp.component, [cellA, cellB])
        if (reason) {
            deferred.push({component: comp.component, varies: w.varies, reason})
            continue
        }
        const outside = [cellA, cellB].filter((c) => !inSweptDomain(parseCell(c)))
        if (outside.length)
            outsideSweptDomain.push({component: comp.component, varies: w.varies, cells: outside})
        covered.push({component: comp.component, varies: w.varies, cells: [cellA, cellB]})
    }
}
console.log(
    `production coverage: ${covered.length} witnesses inside the swept domain ` +
        `(already production-confirmed by headless-sweep.md); ` +
        `${outsideSweptDomain.length} outside; ${deferred.length} deferred (labelled)`
)
for (const o of outsideSweptDomain)
    console.log(`  ! outside the swept domain: ${o.component} varying ${o.varies}`)


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
for (const row of covered) {
    const s = witnessStatus.get(row.component) ?? {confirmed: 0, reasons: new Set()}
    s.confirmed++
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
    "the sensitive region: \"depends on Y only when Z ∈ S\"). This analysis",
    `computes this on the executable spec over its full modelled domain`,
    `(min ≤ max; ${analysis.domain.cells_evaluated.toLocaleString("en-US")} evaluations — the`,
    "cross-product IS materialisable on the spec, unlike on production).",
    "Each dependence carries a **witness** (a concrete cell pair",
    "proving it) through the real WASM checker/gates/tally where the fixtures",
    "can represent it, and labels every witness it cannot reach.",
    "",
    "**What this analysis cannot see.** This analyses the *spec*: in",
    "regions no validation lane has reached, its claims describe the",
    "transcription, not production — and a dependency production has that the",
    "transcription missed is invisible here by construction. The instruments",
    "for that residue are the witness checks below (per-claim evidence",
    "tiers), the consumer/input censuses and scope boundaries",
    "(`README.md` in this directory), and the browser lane for the",
    "filter/reachability components (pending — see the deferred labels).",
    "",
    `**Production coverage: ${covered.length} witnesses lie inside the ` +
        `exhaustively-swept headless domain, so production has already been ` +
        `compared against this spec on their cells (headless-sweep.md); ` +
        `${outsideSweptDomain.length} outside it; ${deferred.length} deferred ` +
        `with labels.**`,
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
    "unvalidated. The *browser-pending* deferrals are handled by the",
    "stage-2 witness lane, `browser-witnesses.mjs` → `browser-witnesses.md`,",
    "which drives them through the real booth; the witnesses its recipes",
    "cannot reach stay labelled there.",
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
if (outsideSweptDomain.length) {
    md.push("## OUTSIDE THE SWEPT DOMAIN (coverage not inherited)", "")
    for (const o of outsideSweptDomain) {
        md.push(`- ${o.component} varying ${o.varies}: ${JSON.stringify(o.cells)}`)
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
            production_coverage: {covered, deferred, outside_swept_domain: outsideSweptDomain},
        },
        null,
        2
    ) + "\n"
)
console.log("wrote effect-dependencies.md and effect-dependencies.recorded.json")
if (outsideSweptDomain.length) process.exitCode = 1
