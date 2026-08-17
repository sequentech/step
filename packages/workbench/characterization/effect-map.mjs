// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The effect map — the human-facing projection of the validated dependency
// ledger (`effect-dependencies.recorded.json`): a causal diagram of the
// mapping plus per-knob cards. Pure JSON → markdown; no wasm, no browser.
//
// The diagram is a deterministic structural causal model: each node a
// variable, each edge "appears in the parent list of the node's structural
// equation" (the spec function named on the node). The TOPOLOGY is the
// spec's own factorization (intensional — how the code composes); the
// generator CHECKS it against the extensional ledger both ways:
//   - every recorded dependence must have a graph path (a violation fails
//     the run — the diagram may never under-report);
//   - graph paths with NO recorded dependence are emitted as their own
//     table: functional cancellations the topology cannot see (the
//     determinism caveat — d-separation is sound, not complete). The
//     flagship: five of the six policies have a path to the tally through
//     the checker record, and all five provably compose to nothing.
//
// Run:  node characterization/effect-map.mjs   (from packages/workbench)

import {readFileSync, writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"

const here = path.dirname(fileURLToPath(import.meta.url))
const deps = JSON.parse(
    readFileSync(path.join(here, "effect-dependencies.recorded.json"), "utf8")
)

// ---------------------------------------------------------------------------
// Aggregation: ledger dimensions → diagram input nodes; ledger components →
// diagram effect nodes. Coarse on purpose — the cards and the matrix carry
// the fine grain.
// ---------------------------------------------------------------------------
const DIM_NODE = {
    invalid_vote_policy: "inv",
    blank_vote_policy: "blank",
    over_vote_policy: "over",
    under_vote_policy: "under",
    duplicated_rank_policy: "dupP",
    preference_gaps_policy: "gapP",
    min_votes: "bounds",
    max_votes: "bounds",
    regulars: "sel",
    blank_marker: "sel",
    explicit_invalid: "sel",
    decline: "dec",
    duplicate_ranks: "ranks",
    rank_gaps: "ranks",
}
const INPUT_LABEL = {
    inv: "invalid_vote_policy",
    blank: "blank_vote_policy",
    over: "over_vote_policy",
    under: "under_vote_policy",
    dupP: "duplicated_rank_policy",
    gapP: "preference_gaps_policy",
    bounds: "min_votes / max_votes",
    sel: "selections (regulars, blank marker, invalid flag)",
    dec: "decline",
    ranks: "ranked state (duplicates, gaps)",
}
const compNode = (name) => {
    if (name.endsWith("∈ errors") || name.endsWith("∈ alerts")) return "rec"
    if (name.endsWith("∈ inline.voting")) return "iv"
    if (name.endsWith("∈ inline.review")) return "ir"
    if (name === "gate.hard" || name === "gate.soft") return "gates"
    if (name === "dialog") return "dlg"
    if (name === "reachability") return "reach"
    if (name === "tally") return "tally"
    throw new Error(name)
}
const EFFECT_LABEL = {
    rec: "checker record (emissions)",
    gates: "gate pair (hard, soft)",
    dlg: "dialog",
    iv: "inline (voting, touched)",
    ir: "inline (review)",
    reach: "reachability",
    tally: "tally class",
}

// Aggregate support: input node → effect node, from the ledger.
const support = new Set()
for (const c of deps.components) {
    if (c.constant) continue
    const eff = compNode(c.component)
    for (const dim of c.support) support.add(`${DIM_NODE[dim]}→${eff}`)
}

// ---------------------------------------------------------------------------
// Topology: the spec's factorization (each edge = the parent appears in the
// named structural equation). Checked against `support` below.
// ---------------------------------------------------------------------------
const EDGES = [
    // emissions() — checker.rs transcription
    ["sel", "rec"], ["bounds", "rec"], ["ranks", "rec"],
    ["inv", "rec"], ["blank", "rec"], ["over", "rec"], ["under", "rec"],
    // inlineViews() — filterErrorList: record + the four consulted policies
    ["rec", "iv"], ["rec", "ir"],
    ["inv", "iv"], ["blank", "iv"], ["over", "iv"], ["under", "iv"],
    ["inv", "ir"], ["blank", "ir"], ["over", "ir"], ["under", "ir"],
    // hardGate()/softGate() — record + independent re-derivations (the
    // drift-prone second expressions: S4's home)
    ["rec", "gates"], ["sel", "gates"], ["bounds", "gates"],
    ["inv", "gates"], ["blank", "gates"], ["over", "gates"],
    ["under", "gates"], ["dupP", "gates"], ["gapP", "gates"],
    // dialog = projection
    ["gates", "dlg"],
    // reachability()
    ["over", "reach"], ["bounds", "reach"], ["sel", "reach"],
    // classify() — record (errors-nonempty) + selection class + decline
    ["rec", "tally"], ["sel", "tally"], ["dec", "tally"],
]

// Reachability over the topology (inputs → effect nodes).
const succ = {}
for (const [a, b] of EDGES) (succ[a] ??= []).push(b)
const reaches = (from, to) => {
    const seen = new Set([from])
    const q = [from]
    while (q.length) {
        const n = q.shift()
        if (n === to) return true
        for (const m of succ[n] ?? []) if (!seen.has(m)) (seen.add(m), q.push(m))
    }
    return false
}

// Check 1: every recorded dependence has a path. (Never under-report.)
const inputNodes = [...new Set(Object.values(DIM_NODE))]
const effectNodes = ["rec", "gates", "dlg", "iv", "ir", "reach", "tally"]
const missing = []
for (const s of support) {
    const [a, b] = s.split("→")
    if (!reaches(a, b)) missing.push(s)
}
if (missing.length) {
    console.error("TOPOLOGY UNDER-REPORTS the ledger:", missing)
    process.exit(1)
}
// Check 2: paths with no recorded dependence = functional cancellations.
const cancellations = []
for (const a of inputNodes) {
    for (const b of effectNodes) {
        if (reaches(a, b) && !support.has(`${a}→${b}`)) {
            cancellations.push([a, b])
        }
    }
}

// ---------------------------------------------------------------------------
// Emit
// ---------------------------------------------------------------------------
const md = []
md.push(
    "<!--",
    " SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>",
    "",
    "SPDX-License-Identifier: AGPL-3.0-only",
    "-->",
    "",
    "# The effect map — the validation mapping as a causal diagram",
    "",
    "Generated by `characterization/effect-map.mjs` from the validated",
    "dependency ledger (`effect-dependencies.recorded.json`); do not edit by",
    "hand.",
    "",
    "**What this is.** The vote-validation mapping drawn as a deterministic",
    "*structural causal model*: each node a variable, each arrow \"appears in",
    "the parent list of that node's structural equation\" (the spec function",
    "in the node's box). Setting a policy in the admin portal, or a voter",
    "forming a selection, is an intervention on an input node; the effects",
    "that can respond are the descendants. The topology is the spec's own",
    "factorization, and the generator checks it against the exhaustively",
    "computed ledger: it can never under-report a dependence (checked on",
    "every run), and paths that carry no real dependence are listed below as",
    "*functional cancellations*.",
    "",
    "Evidence behind every arrow: the pipeline of",
    "[`effect-dependencies.md`](effect-dependencies.md) (witnesses),",
    "[`headless-sweep.md`](headless-sweep.md) (exhaustive, 138,240 cells),",
    "[`browser-witnesses.md`](browser-witnesses.md) and",
    "[`quotient-validate.md`](quotient-validate.md) (booth) — zero",
    "disagreements; residual labels live in those artifacts.",
    "",
    "```mermaid",
    "flowchart LR",
    "  subgraph config [contest configuration]",
    "    inv[invalid_vote_policy]",
    "    blank[blank_vote_policy]",
    "    over[over_vote_policy]",
    "    under[under_vote_policy]",
    "    dupP[duplicated_rank_policy]",
    "    gapP[preference_gaps_policy]",
    "    bounds[min_votes / max_votes]",
    "  end",
    "  subgraph vote [vote state]",
    "    sel[selections: regulars + markers + flag]",
    "    ranks[ranked state: duplicates / gaps]",
    "    dec[decline]",
    "  end",
    "  rec[checker record - emissions - checker.rs]",
    "  gates[gate pair hard/soft - voting_screen.rs]",
    "  dlg((dialog))",
    "  iv((inline at voting - filterErrorList))",
    "  ir((inline at review - filterErrorList))",
    "  reach((reachability - Question/Answer + reducers))",
    "  tally((tally class - classify_ballot))",
    "  sel --> rec",
    "  bounds --> rec",
    "  ranks --> rec",
    "  inv --> rec",
    "  blank --> rec",
    "  over --> rec",
    "  under --> rec",
    "  rec --> iv",
    "  rec --> ir",
    "  inv -- MUTE under allowed --> iv",
    "  inv -- MUTE under allowed --> ir",
    "  blank --> iv",
    "  blank --> ir",
    "  over -- keep-list carve-out --> iv",
    "  over -- carve-out; hint hidden at review --> ir",
    "  under -- WARN_ONLY_IN_REVIEW hides at voting --> iv",
    "  under --> ir",
    "  rec --> gates",
    "  sel -- re-derived counts --> gates",
    "  bounds -- re-derived bounds --> gates",
    "  inv --> gates",
    "  blank --> gates",
    "  over --> gates",
    "  under -- n > 0 guard: S4 --> gates",
    "  dupP --> gates",
    "  gapP --> gates",
    "  gates --> dlg",
    "  over -- DISABLE --> reach",
    "  bounds --> reach",
    "  sel -- marker clearing --> reach",
    "  rec -- errors present? --> tally",
    "  sel -- selection class --> tally",
    "  dec --> tally",
    "  classDef effect fill:#1f7a8c,color:#fff",
    "  classDef inter fill:#555,color:#fff",
    "  class dlg,iv,ir,reach,tally effect",
    "  class rec,gates inter",
    "```",
    "",
    "Round nodes are the four **effect categories** (what is observable);",
    "rectangles inside the mapping are the two **checkable intermediates**",
    "(the record and the gate pair — validated against the WASM but not",
    "directly observable; VALIDATION_LOGIC_DISTILLATION.md §1). Edge labels",
    "carry the crisp guards; the full conditional-independence set (317",
    "restrictions) is in the ledger.",
    "",
    "**How to read S1 off the diagram:** the inputs that can change the",
    "*tally* are `sel`, `dec`, and — through the record — `blank`, `bounds`,",
    "`ranks`. For a voter to notice, the same change must reach *dialog* or",
    "*inline*. The MUTE edge (`invalid = allowed`) severs the record's",
    "messages on their way into both inline nodes except for the two",
    "carve-outs, and the gate clauses that would raise a dialog are exactly",
    "the ones the silent-prone configurations switch off — outcome-path",
    "open, signal-paths severed. That geometry is the silent-discount",
    "family; the cells realizing it are `no-silent-discount.md`.",
    ""
)

md.push(
    "## Functional cancellations — paths that provably carry nothing",
    "",
    "A path in the diagram means influence is *plumbed*, not that it ever",
    "changes the effect: determinism lets compositions cancel, and the",
    "exhaustive ledger proves these do. The flagship result — the",
    "\"dimmer-switch\" theorem — reads off in two parts: `duplicated_rank` /",
    "`preference_gaps` policies have **no path to the tally at all**",
    "(structurally disconnected from the count), and the three policies",
    "below have a path through the checker record yet **provably never",
    "change the count** — leaving `blank_vote_policy` as the only policy the",
    "tally depends on. The policies dim the signalling around a wall whose",
    "position they cannot move.",
    "",
    "| input | effect it can reach but provably never changes |",
    "|---|---|",
    ...cancellations.map(
        ([a, b]) => `| ${INPUT_LABEL[a]} | ${EFFECT_LABEL[b]} |`
    ),
    ""
)

// ---------------------------------------------------------------------------
// Per-knob cards
// ---------------------------------------------------------------------------
const POLICY_DIMS = [
    "invalid_vote_policy",
    "blank_vote_policy",
    "over_vote_policy",
    "under_vote_policy",
    "duplicated_rank_policy",
    "preference_gaps_policy",
]
md.push(
    "## Per-knob cards — what each policy provably does and does not control",
    "",
    "Each card is a column-slice of the validated support matrix",
    "(`effect-dependencies.md`), grouped by effect category. \"Never\" means",
    "proven over the whole modelled domain, exhaustively on the spec and",
    "per the pipeline against production.",
    ""
)
const CATS = [
    ["checker record", (n) => n.endsWith("∈ errors") || n.endsWith("∈ alerts")],
    ["inline (voting)", (n) => n.endsWith("∈ inline.voting")],
    ["inline (review)", (n) => n.endsWith("∈ inline.review")],
    ["gates / dialog", (n) => ["gate.hard", "gate.soft", "dialog"].includes(n)],
    ["reachability", (n) => n === "reachability"],
    ["tally", (n) => n === "tally"],
]
for (const dim of POLICY_DIMS) {
    const can = []
    const cannot = []
    for (const [cat, match] of CATS) {
        const comps = deps.components.filter((c) => !c.constant && match(c.component))
        const hits = comps.filter((c) => c.support.includes(dim))
        if (hits.length) {
            can.push(
                `**${cat}**: ` +
                    hits.map((c) => c.component.split(" ")[0]).filter((v, i, a) => a.indexOf(v) === i).join(", ")
            )
        } else if (comps.length) {
            cannot.push(cat)
        }
    }
    const conds = []
    for (const c of deps.components) {
        for (const r of c.restrictions) {
            if (r.depends_on === dim && POLICY_DIMS.includes(r.only_when)) {
                conds.push(
                    `${c.component} responds to it only when \`${r.only_when}\` ∈ {${r.in.join(", ")}}`
                )
            }
        }
    }
    md.push(
        `### \`${dim}\``,
        "",
        `Can change — ${can.join("; ")}.`,
        "",
        `**Provably never changes** — ${cannot.join(", ") || "(nothing: it reaches every category)"}.`,
        ""
    )
    if (conds.length) {
        md.push("Conditional (policy-level):", "", ...conds.map((x) => `- ${x}`), "")
    }
}

writeFileSync(path.join(here, "effect-map.md"), md.join("\n") + "\n")
console.log(
    `wrote effect-map.md (${support.size} aggregate dependences drawn, ` +
        `${cancellations.length} functional cancellations, ${POLICY_DIMS.length} cards)`
)
