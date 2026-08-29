// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// General DOM validator — the evidence for the spec's browser-only half.
//
// The headless runners predict inline visibility (`spec.inlineViews` — the
// touched voting screen AND the review screen) and reachability
// (`spec.reachability`) but cannot observe them — `filterErrorList`, the
// input disable and the marker clearing are TypeScript/React. This validates
// those predictions against the REAL DOM, reload-free, by:
//   - reading each rule's recorded JSON for the per-cell prediction
//     (the Rust spec's inline views, dialog and reachability), and
//   - driving the booth (panel config → cast state → observe) via
//     `browser-harness.mjs`, then comparing. Every cell observes the TOUCHED
//     voting screen (observeBooth's deterministic tick-untick); the
//     untouched view is ASSERTED empty on every cell before arming (the
//     untouched-clear constant — a run fails if anything renders there).
//
// It drives config through the PANEL (not dispatch) on purpose: a panel
// regression would make the config miss the booth and the DOM diverge from
// the prediction, so this doubles as the reviewer-path check REPRODUCE.md
// relies on — across every cell, not just the headline ones.
//
// Comparison is over UNIQUE message keys: the booth may render a message
// as both an error box and an alert box, but the set of visible keys is
// what "does the voter see it?" turns on.
//
// Requires the dev server on :5173. Covers all seven rules: the five plurality
// rules (over-vote, min-vote, blank, under-vote, invalid) on the explicit-
// blank-invalid fixture, and the two preferential rules (duplicate-rank,
// preference-gaps) on the IRV fixture with ranked selection. Adding a rule is a
// RULES entry (+ a RULE_SPECS entry keyed by its recorded-JSON name).

import {createRequire} from "node:module"
import {writeFileSync} from "node:fs"
import {performance} from "node:perf_hooks"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {loadSnapshot} from "./browser-harness.mjs"
import {specFixed} from "./rust-spec.mjs"
import {inCertifiedDomain} from "./domain.mjs"
import {RULE_SPECS, RULE_ROWS, contestAndVoter, observeBooth, isReached} from "./rule-specs.mjs"

const require = createRequire("C:/work/projects/step/packages/")
const {chromium} = require("playwright")

const here = path.dirname(fileURLToPath(import.meta.url))
const base = "http://localhost:5173"

// Each rule names its fixture; the fixture fixes the snapshot to load and the
// election id its contest lives under. Plurality rules ride the
// explicit-blank-invalid fixture; the two preferential rules the IRV one.
const FIXTURES = {
    plurality: {
        snapshot: "bundled:explicit-blank-invalid",
        election: "44444444-4444-4444-4444-444444444003",
    },
    irv: {
        snapshot: "bundled:instant-runoff-3cand",
        election: "11111111-1111-1111-1111-111111111003",
    },
}

const uniq = (xs) => [...new Set(xs)].sort()

// Every prediction in this runner comes from the TYPED RUST SPEC
// (`../validation-spec` via `emit-grid`) — inline views at both observation
// points, the dialog, and reachability. Cells are evaluated in one batch
// before the browser work starts and looked up by the runner as it goes.
//
// INJECTION STATUS: COMPLETE (mirrors headless-sweep.mjs) — the gates,
// decode, and the booth's message filter all run the rationalized
// implementation, so every prediction here comes from f_fixed.
//
// This compares the DOM against the artifact the project ships, rather than
// against inline re-derived from each cell's own recorded emissions. The two
// coincide wherever the sweep certifies production ≡ f_fixed per component,
// which is checked below (`outsideSweptDomain`) rather than assumed.
const predictions = new Map()
const predKey = (ruleName, i) => `${ruleName}#${i}`
const outsideSweptDomain = []

// The browser-driving specs (contest, config, selection, landmark) come from
// rule-specs.mjs — the single source shared with no-silent-discount. Here we
// add only the validation extras: the recorded rows to validate against, the
// predicted input-constraint, and a display label.
const RULES = [
    {
        name: "over-vote",
        ...RULE_SPECS["overvote-rule"],
        rows: RULE_ROWS["overvote-rule"],
        label: (r) => `${r.over_vote_policy} × ${r.invalid_vote_policy} × ${r.state}`,
        configNote: "*config* = `over_vote_policy` × `invalid_vote_policy`.",
    },
    {
        name: "min-vote",
        ...RULE_SPECS["minvote-rule"],
        rows: RULE_ROWS["minvote-rule"],
        label: (r) => `min=${r.min_votes} × ${r.invalid_vote_policy} × ${r.state}`,
        configNote:
            "*config* = `min_votes` × `invalid_vote_policy` — min-vote is a fixed " +
            "rule, so the `min_votes` bound is the knob, not a policy.",
    },
    {
        name: "blank",
        ...RULE_SPECS["blank-rule"],
        rows: (() => {
            const rows = RULE_ROWS["blank-rule"].filter((r) => r.state !== "explicit_invalid")
            // Browser-only marker-exclusivity cell (no headless analogue: the
            // clear is a reducer step the recording bypasses). Selecting a
            // regular THEN the blank marker must collapse to {marker only} — the
            // blank marker clears the regular, the mirror of the invalid
            // `marker_plus` cell (which does NOT clear). Its effect columns are
            // the resulting {marker only} state's, borrowed from that cell; the
            // new information is `reachable = no (cleared)`.
            const base = rows.find(
                (r) =>
                    r.state === "marker_only" &&
                    r.blank_vote_policy === "allowed" &&
                    r.invalid_vote_policy === "allowed"
            )
            rows.push({...JSON.parse(JSON.stringify(base)), state: "regular_then_marker"})
            return rows
        })(),
        // Direct evidence the blank marker cleared the co-selected regular:
        // exactly the marker survives (Yes at index 0 deselected, the blank
        // marker at index 2 set) in the Referendum's [Yes, No, marker] order.
        clearedOk: (obs) => obs.formed === 1 && obs.selected[0] === -1 && obs.selected[2] === 0,
        label: (r) => `${r.blank_vote_policy} × ${r.invalid_vote_policy} × ${r.state}`,
        configNote:
            "*config* = `blank_vote_policy` × `invalid_vote_policy`. The " +
            "`explicit_invalid` state is headless-only — this contest has no " +
            "invalid marker to set the flag through the booth — so it lives in " +
            "the partial table, not here. The `regular_then_marker` row is a " +
            "browser-only marker-exclusivity check: a regular then the blank " +
            "marker collapses to {marker only} (`no (cleared)`) — the mirror of " +
            "the invalid `marker_plus` cell, which does not clear.",
    },
    {
        name: "undervote",
        ...RULE_SPECS["undervote-rule"],
        rows: RULE_ROWS["undervote-rule"],
        label: (r) => `${r.under_vote_policy} × ${r.invalid_vote_policy} × ${r.state}`,
        configNote: "*config* = `under_vote_policy` × `invalid_vote_policy`.",
    },
    {
        name: "invalid",
        ...RULE_SPECS["invalid-rule"],
        rows: RULE_ROWS["invalid-rule"].filter((r) => r.state !== "flag_only"),
        label: (r) => `${r.invalid_vote_policy} × ${r.state}`,
        configNote:
            "*config* = `invalid_vote_policy`. The `flag_only` state is " +
            "headless-only — the booth sets the invalid flag only via the marker " +
            "(the `marker` state it converges with) — so it lives in the partial " +
            "table, not here.",
    },
    {
        name: "duplicate-rank",
        ...RULE_SPECS["duprank-rule"],
        fixture: "irv",
        rows: RULE_ROWS["duprank-rule"],
        label: (r) => `${r.duplicated_rank_policy} × ${r.invalid_vote_policy} × ${r.state}`,
        configNote:
            "*config* = `duplicated_rank_policy` × `invalid_vote_policy` on the " +
            "IRV *Favourite fruit* contest (Apple/Banana/Cherry, ranked). " +
            "*state*: `valid_full` = ranks 1,2,3; `duplicate` = two candidates at " +
            "rank 1.",
    },
    {
        name: "preference-gaps",
        ...RULE_SPECS["prefgaps-rule"],
        fixture: "irv",
        rows: RULE_ROWS["prefgaps-rule"],
        label: (r) => `${r.preference_gaps_policy} × ${r.invalid_vote_policy} × ${r.state}`,
        configNote:
            "*config* = `preference_gaps_policy` × `invalid_vote_policy` on the " +
            "IRV *Favourite fruit* contest (Apple/Banana/Cherry, ranked). " +
            "*state*: `valid_full` = ranks 1,2,3; `gap` = ranks 1 then 3, " +
            "skipping rank 2.",
    },
]

const browser = await chromium.launch({channel: "chrome", headless: true})
const page = await browser.newPage()

const short = (xs) =>
    !xs || xs.length === 0
        ? "—"
        : uniq(xs).map((m) => m.replace(/^errors\.\w+\./, "")).join("<br>")

const results = []
const untouchedViolations = []
const t0 = performance.now()
// ---------------------------------------------------------------------------
// Predict every cell up front, in one batch: `emit-grid` is a subprocess, so
// one call for the whole suite rather than one per cell. The same pass checks
// each cell against the swept headless domain — the sweep is what licenses
// comparing the DOM against SPEC-derived inline rather than inline re-derived
// from that cell's own observed emissions, so a cell outside it would be
// relying on a licence the sweep never granted.
// ---------------------------------------------------------------------------
{
    const batch = []
    const keys = []
    for (const rule of RULES)
        for (const [ri, r] of rule.rows.entries()) {
            const config = rule.specConfig(r)
            const voteState = rule.voteState(r)
            batch.push({config, voteState})
            keys.push(predKey(rule.name, ri))
            // A row whose requested state cannot form records the effects of
            // the state that does (see `effectsVoteState`), so it needs a
            // second prediction.
            const effectsVs = rule.effectsVoteState?.(r)
            if (effectsVs) {
                batch.push({config, voteState: effectsVs})
                keys.push(predKey(rule.name, ri) + "@effects")
            }
            const why = inCertifiedDomain({config, voteState})
            if (why)
                outsideSweptDomain.push({rule: rule.name, cell: rule.label(r), reason: why})
        }
    const outs = specFixed(batch)
    outs.forEach((o, i) => predictions.set(keys[i], o))
    console.log(
        `predicted ${outs.length} cells from the Rust spec; ` +
            `${outsideSweptDomain.length} outside the swept domain`
    )
    for (const c of outsideSweptDomain)
        console.log(`  ! ${c.rule} ${c.cell}: ${c.reason}`)
}

for (const rule of RULES) {
    // Reload per rule for a clean baseline (also switches fixture/snapshot for
    // the preferential rules). Panel overrides are ephemeral and per-contest,
    // and several rules share a contest (over-vote/invalid on Council;
    // min-vote/blank/undervote on Referendum), so a policy one rule leaves
    // unset would otherwise inherit the previous rule's value. Reloading
    // between rules (once each) keeps the per-cell reload-free speed.
    const {snapshot, election} = FIXTURES[rule.fixture ?? "plurality"]
    await loadSnapshot(page, base, snapshot)
    const {contestId, voterId} = await contestAndVoter(page, election, {
        flag: rule.contestFlag,
        counting: rule.contestCounting,
    })
    for (const [ri, r] of rule.rows.entries()) {
        const obs = await observeBooth(page, {electionId: election, contestId, voterId, spec: rule, cell: r})

        // The untouched-clear constant: nothing may render before the touch
        // is armed, whatever the checker emitted for this cell.
        if ((obs.inlineUntouched ?? []).length > 0) {
            untouchedViolations.push({rule: rule.name, cell: rule.label(r), shown: obs.inlineUntouched})
            console.log(`✗ untouched view NOT empty: ${rule.name} ${rule.label(r)}: ${JSON.stringify(obs.inlineUntouched)}`)
        }

        // The spec predicts a state cannot form by one of two prevention
        // mechanisms: an input disable ("inputs_disabled") or marker exclusivity
        // ("marker_cleared"). Either way the cell is `constrained`. Computed
        // uniformly from the same cell definitions the runners feed spec.f.
        const pred = predictions.get(predKey(rule.name, ri))
        // Effect columns come from the state that forms; reachability from
        // the state that was requested.
        const eff = predictions.get(predKey(rule.name, ri) + "@effects") ?? pred
        const reach = pred.reachability
        const constraintPred = reach === "yes" ? null : reach
        const constrained = constraintPred !== null
        const domReachable = isReached(rule, obs, r)
        const reachableOk = domReachable === !constrained

        // Inline is validated at BOTH observation points. The voting view is
        // read before Next is clicked, so it is comparable whenever the state
        // formed; the review view additionally requires that no blocking
        // dialog preempts review (there the dialog is the signal, validated
        // by dialogOk). Neither is comparable for an unreachable state — the
        // observed views then belong to whatever state actually formed, and
        // the constraint is the signal, validated by reachableOk.
        const votingComparable = !constrained
        const votingOk =
            !votingComparable ||
            JSON.stringify(uniq(obs.inlineAtVote ?? [])) ===
                JSON.stringify(uniq(pred.inline.voting))
        const inlineComparable = !constrained && obs.dialog !== "blocking"
        const inlineOk =
            !inlineComparable ||
            JSON.stringify(uniq(obs.inlineAtReview ?? [])) ===
                JSON.stringify(uniq(pred.inline.review))
        const dialogOk = constrained || obs.dialog === pred.dialog
        // Direct DOM evidence for the constraint — a second signal beyond
        // behavioural non-reachability, specific to the mechanism: the disable
        // policy's (max+1)th control must carry `disabled` (obs.constraintProbe),
        // and marker exclusivity must leave exactly the marker selected
        // (rule.clearedOk). Unconstrained cells are vacuously ok.
        const constraintDirectOk =
            constraintPred === "inputs_disabled"
                ? obs.constraintProbe === true
                : constraintPred === "marker_cleared"
                  ? rule.clearedOk(obs)
                  : true
        const ok = votingOk && inlineOk && dialogOk && reachableOk && constraintDirectOk

        // Observation-derived silent-discount marker: discarded, reachable, and
        // no signal at either casting point (no dialog, nothing inline at the touched
        // voting screen or at review).
        const silent =
            eff.tally === "ImplicitInvalid" &&
            obs.dialog === "none" &&
            (obs.inlineAtVote ?? []).length === 0 &&
            (obs.inlineAtReview ?? []).length === 0 &&
            domReachable

        results.push({
            rule: rule.name,
            config: rule.label(r).replace(` × ${r.state}`, ""),
            state: r.state,
            errors: eff.emissions.errors,
            alerts: eff.emissions.alerts,
            inlineVoting: short(obs.inlineAtVote),
            inlineReview: obs.dialog === "blocking" ? "(blocked)" : short(obs.inlineAtReview),
            hard: eff.gate.hard,
            soft: eff.gate.soft,
            reachable: domReachable,
            constraintKind: constraintPred,
            constraintProbe: obs.constraintProbe,
            dialog: obs.dialog,
            tally: eff.tally,
            silent,
            ok,
        })
        if (!ok) {
            console.log(
                `✗ ${rule.name} ${rule.label(r)}: ` +
                    `inline@voting=${JSON.stringify(uniq(obs.inlineAtVote ?? []))} ` +
                    `predV=${JSON.stringify(uniq(pred.inline.voting))} ` +
                    `inline@review=${JSON.stringify(uniq(obs.inlineAtReview ?? []))} ` +
                    `predR=${JSON.stringify(uniq(pred.inline.review))} ` +
                    `dialog=${obs.dialog}/${pred.dialog} reachable=${domReachable}/${!constrained} ` +
                    `constraint=${constraintPred} probe=${obs.constraintProbe} selected=${JSON.stringify(obs.selected)}`
            )
        }
    }
    const rc = results.filter((x) => x.rule === rule.name)
    console.log(
        `${rule.name}: ${rc.filter((x) => x.ok).length}/${rc.length} DOM-✓, ` +
            `${rc.filter((x) => x.silent).length} silent`
    )
}
await browser.close()

const totalMs = Math.round(performance.now() - t0)
const passed = results.filter((x) => x.ok).length
const allOk = passed === results.length && untouchedViolations.length === 0
console.log(
    `\n${passed}/${results.length} cells validated against the real DOM in ${totalMs}ms ` +
        `(~${Math.round(totalMs / results.length)}ms/cell). all DOM-✓: ${allOk} ` +
        `(untouched-view violations: ${untouchedViolations.length})`
)

// --- complete tables (one per rule) -----------------------------------------
const reachCell = (x) =>
    x.reachable
        ? "yes"
        : x.constraintKind === "inputs_disabled"
          ? "**no** (disabled)"
          : x.constraintKind === "marker_cleared"
            ? "**no** (cleared)"
            : "**no**"
const fmtRow = (x) =>
    `| ${x.silent ? "**⚠** " : ""}${x.config} | ${x.state} | ` +
    `${short(x.errors)} | ${short(x.alerts)} | ${x.inlineVoting} | ${x.inlineReview} | ` +
    `${x.hard ? "**block**" : "—"} | ${x.soft ? "dialog" : "—"} | ` +
    `${reachCell(x)} | ${x.tally} | ${x.ok ? "✓" : "**✗**"} |`
const md = [
    "<!--",
    " SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>",
    "",
    "SPDX-License-Identifier: AGPL-3.0-only",
    "-->",
    "",
    "# DOM-validated complete tables",
    "",
    "Generated by `characterization/dom-validate.mjs`; do not edit by hand.",
    "",
    "The **complete** view — every value is an OBSERVATION, and a **superset**",
    "of the partial rule table's columns. The WASM-observed columns are exactly",
    "the partial's (*errors*, *alerts*, *hard/soft gate*, *tally*); the complete",
    "view ADDS the browser-only observations the partial cannot show —",
    "*inline (voting)* (inline visibility on the voting screen once the contest",
    "is touched; every cell observes the touched voting screen via a deterministic",
    "tick-untick; before arming, the untouched view is asserted EMPTY on",
    "every cell — the untouched-clear constant, enforced here rather than",
    "recorded), *inline (review)* (inline visibility at the decisive",
    "review screen, where the untouched-clear does not apply) and *reachable*",
    "(the input constraint; `no` = the state cannot be formed; `no (disabled)` =",
    "also confirmed by the (max+1)th control carrying `disabled` in the DOM;",
    "`no (cleared)` = a marker cleared a co-selected candidate, collapsing the",
    "state) — plus the observation-derived **⚠**",
    "(discarded ∧ reachable ∧ no signal at either casting point). The single",
    "*matches spec?* column subsumes the partial's `pred?` and extends it to the",
    "browser observations (including the observed dialog vs the gates): ✗ = spec and",
    "DOM disagree. The injection is complete (`headless-sweep.md`): the gates,",
    "decode and the booth's message filter all run the rationalized",
    "implementation, so every prediction comes from `f_fixed`.",
    "`(blocked)` inline means a blocking dialog preempts review —",
    "the dialog is the signal there. For an unreachable state both inline",
    "columns show what the state that ACTUALLY formed renders (the *reachable*",
    "column qualifies it); they are compared against the spec only for",
    "reachable cells. Every column is common and defined here",
    "except *config*, which packs each rule's own policy knobs into one cell —",
    "its meaning is noted under each rule heading below.",
    "",
]
for (const rule of RULES) {
    const rc = results.filter((x) => x.rule === rule.name)
    md.push(
        `## ${rule.name}`,
        "",
        rule.configNote,
        "",
        "| config | state | errors | alerts | inline (voting) | inline (review) | hard gate | soft gate | reachable | tally | matches spec? |",
        "|---|---|---|---|---|---|---|---|---|---|---|",
        ...rc.map(fmtRow),
        ""
    )
}
writeFileSync(path.join(here, "dom-validate.md"), md.join("\n") + "\n")
writeFileSync(
    path.join(here, "dom-validate.recorded.json"),
    JSON.stringify({cells: results, passed, total: results.length, all_ok: allOk}, null, 2) + "\n"
)
console.log("wrote dom-validate.md and dom-validate.recorded.json")
if (!allOk) process.exitCode = 1
