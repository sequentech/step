// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Per-rule tables — DOCUMENTATION, not evidence.
//
// This renders what `f_fixed` — production's own validation rules
// (`sequent-core/src/validation.rs`), served through `emit-grid` — says
// each rule does across its grid. Nothing here observes
// production, compares anything, or can fail: it is a reading aid, the
// per-rule view of a mapping whose fidelity is established elsewhere.
//
// WHERE THE EVIDENCE IS. That these tables describe production is the
// sweep's claim, not theirs: `headless-sweep.md` compares production
// against this same spec on every cell of the representable headless
// domain, and all 248 cells rendered here lie inside it (checked below).
// The booth-side columns — inline visibility, reachability — are in
// `dom-validate.md`, which drives every one of these cells through a real
// browser.
//
// These files used to be seven runners that each re-observed production
// and carried a `pred?` column. That column is gone because the comparison
// is gone: it was redundant with the sweep, and having it here made the
// evidence story look like it had seven independent checks that it did
// not (EVIDENCE_RESTRUCTURE.md, step 6).
//
// Run:  node characterization/rule-tables.mjs   (from packages/workbench)

import {writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {RULE_SPECS, RULE_ROWS} from "./rule-specs.mjs"
import {specFixed} from "./rust-spec.mjs"
import {representable} from "./cell.mjs"

const here = path.dirname(fileURLToPath(import.meta.url))
const short = (xs) =>
    xs.length === 0 ? "—" : xs.map((m) => m.replace(/^errors\.\w+\./, "")).join("<br>")

const LEGENDS = {
    "blank-rule": {
        title: "Blank-rule characterization",
        knob: "blank_policy",
        experiment: "**Experiment:** every row is one vote state on the *Referendum* contest\n(Yes / No / explicit-blank marker, `min_votes: 0`, `max_votes: 2`) under\none (blank policy × invalid policy) configuration. States:\n*empty* = nothing selected (the blank condition); *explicit_invalid* =\nnothing selected, explicit-invalid flag set; *marker_only* = the\nexplicit-blank marker alone (counts as a selection — NOT blank at the\nbooth); *one_regular* = one normal candidate (control).\nOver/under policies at defaults.",
    },
    "overvote-rule": {
        title: "Over-vote rule characterization",
        knob: "over_policy",
        experiment: "**Experiment:** every row is one vote state on the *Council seat* contest\n(Ada / Bruno / explicit-invalid marker, `min_votes: 0`, `max_votes: 1`)\nunder one (over-vote policy × invalid policy) configuration. States:\n*empty* = nothing selected; *at_max* = Ada (exactly `max_votes`);\n*over_max* = Ada + Bruno (one over). Blank/under policies at defaults.",
    },
    "undervote-rule": {
        title: "Under-vote rule characterization",
        knob: "under_policy",
        experiment: "**Experiment:** every row is one vote state on the *Referendum* contest\nwith `min_votes` forced to 0 and `max_votes` to 2 (marker ignored; only\nYes / No used), under one (under-vote policy × invalid policy) config.\nStates: *empty* = 0 selections; *under* = 1 (the under-vote zone,\n`min ≤ n < max`); *full* = 2 (exactly max). Blank policy at default.",
    },
    "minvote-rule": {
        title: "Min-vote rule characterization",
        knob: "min_votes",
        experiment: "**Experiment:** min-vote is not a policy — it is the fixed rule\n`count < min_votes → selectedMin error`. Rows vary `min_votes` and the\ninvalid policy on the *Referendum* contest (`max_votes` forced to 3).\nStates: *none* = 0 selections; *one* = 1 regular candidate;\n*marker_only* = the explicit-blank marker alone (which **counts toward**\nmin_votes — the marker-inclusive count).",
    },
    "duprank-rule": {
        title: "Duplicated-rank rule (preferential)",
        knob: "dup_policy",
        experiment: "**Experiment:** every row is one ranked selection on the IRV *Favourite\nfruit* contest (Apple / Banana / Cherry; `selected` = rank, 0-based)\nunder one (`duplicated_rank_policy` × `invalid_vote_policy`) config.\nStates: *valid_full* = ranks 0,1,2 (well-ordered); *duplicate* = ranks\n0,0 (two candidates at rank 1 → DuplicatedPosition). `preference_gaps`\nat default.",
    },
    "prefgaps-rule": {
        title: "Preference-gaps rule (preferential)",
        knob: "gap_policy",
        experiment: "**Experiment:** every row is one ranked selection on the IRV *Favourite\nfruit* contest (Apple / Banana / Cherry; `selected` = rank, 0-based)\nunder one (`preference_gaps_policy` × `invalid_vote_policy`) config.\nStates: *valid_full* = ranks 0,1,2 (well-ordered); *gap* = ranks 0,2\n(skipping rank 1 → PreferenceOrderWithGaps). `duplicated_rank` at default.",
    },
    "invalid-rule": {
        title: "Invalid-vote rule (as subject)",
        knob: "invalid_policy",
        experiment: "**Experiment:** the *Council seat* contest (Ada / Bruno / Null-vote marker,\n`max_votes` forced to 2 to isolate from over-vote) under each\n`invalid_vote_policy`, across five vote states that exercise **both\nroutes to explicit invalidity**: *flag_only* sets the\n`is_explicit_invalid` flag directly (the route the other runners use);\n*marker* selects the null-vote marker candidate (the flag is then derived at\nencode); *marker_plus* adds a regular candidate. *none* / *regular* are\nblank / valid controls.",
    },
}

// The shared provenance block every table carries, so no reader has to
// infer what the numbers are.
const PROVENANCE = [
    "**What this is.** A rendering of the *specification*",
    "(`f_fixed`: production's rules in `sequent-core/src/validation.rs`,",
    "composed by `../validation-adapters`) across this rule's grid —",
    "documentation, not",
    "evidence. No column here is a separate observation of production.",
    "",
    "**Why it describes production anyway.** `headless-sweep.md` compares",
    "production against this same spec on every cell of the representable",
    "headless domain, and every cell below lies inside it. The booth-side",
    "columns — inline visibility at each observation point, reachability —",
    "are validated per cell in `dom-validate.md`.",
    "",
    "**Columns.** *errors* / *alerts* are the checker emissions (message",
    "keys, `errors.implicit.`/`errors.explicit.` prefix stripped);",
    "*hard/soft gate* are the two submission gates (blocking vs dismissible",
    "dialog); *tally* is the per-ballot class the classifier assigns.",
]

let outside = 0
const written = []
for (const [rule, legend] of Object.entries(LEGENDS)) {
    const spec = RULE_SPECS[rule]
    const rows = RULE_ROWS[rule]
    const cells = rows.map((r) => ({config: spec.specConfig(r), voteState: spec.voteState(r)}))
    for (const c of cells) if (representable(c)) outside++
    const out = specFixed(cells)

    const knobOf = (r) =>
        Object.entries(r).find(([k]) => k !== "invalid_vote_policy" && k !== "state")?.[1]
    const hasKnob = rule !== "invalid-rule"
    const header = hasKnob
        ? `| ${legend.knob} | invalid_policy | state | errors | alerts | hard gate | soft gate | tally |`
        : `| invalid_policy | state | errors | alerts | hard gate | soft gate | tally |`
    const sep = "|---".repeat(header.split("|").length - 2) + "|"

    const body = rows.map((r, i) => {
        const e = out[i]
        const lead = hasKnob
            ? `| ${knobOf(r)} | ${r.invalid_vote_policy} | ${r.state} |`
            : `| ${r.invalid_vote_policy} | ${r.state} |`
        return (
            `${lead} ${short(e.emissions.errors)} | ${short(e.emissions.alerts)} | ` +
            `${e.gate.hard ? "**block**" : "—"} | ${e.gate.soft ? "dialog" : "—"} | ${e.tally} |`
        )
    })

    const md = [
        "<!--",
        " SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>",
        "",
        "SPDX-License-Identifier: AGPL-3.0-only",
        "-->",
        "",
        `# ${legend.title}`,
        "",
        "Generated by `characterization/rule-tables.mjs`; do not edit by hand.",
        "",
        legend.experiment,
        "",
        ...PROVENANCE,
        "",
        header,
        sep,
        ...body,
        "",
    ].join("\n")
    writeFileSync(path.join(here, `${rule}.md`), md)
    written.push(`${rule}.md (${rows.length} rows)`)
}

console.log(`rendered ${written.length} per-rule tables from the Rust spec:`)
for (const w of written) console.log(`  ${w}`)
if (outside) {
    console.log(`! ${outside} rendered cells lie OUTSIDE the swept domain —`)
    console.log("  their agreement with production is not evidenced by headless-sweep.md")
    process.exitCode = 1
}
