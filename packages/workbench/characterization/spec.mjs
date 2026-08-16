// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The shared vote-validation specification — the single executable statement
// of the mapping
//
//     f(config, voteState) → one value per surface
//
// given a contest's configuration and a description of what the voter did:
// which checker messages exist (the internal record), what is visible inline
// at each observation point (the voting screen untouched/touched, the review
// screen — the point indexes the inline OUTPUT; it is not an input to the
// mapping), whether the review-transition gates fire, whether the booth UI
// lets the state form at all, and how the tally classifies the ballot.
//
// Everything is here, as one transcription of the production rules rather
// than partial copies: the checker EMISSIONS (`emissions` — which
// `invalid_errors` / `invalid_alerts` checker.rs produces; formerly
// transcribed per rule runner, with the invalid rule's emissions copied into
// every runner), the two gates, the tally classifier, the booth message
// filter (inline visibility), and reachability (which states the booth UI
// refuses to form).
//
// This is the "single artifact" of VALIDATION_LOGIC_DISTILLATION.md §5.1 in
// its first concrete form. It is a transcription of production
// TypeScript/Rust and therefore fallible — its authority comes from being
// CHECKED, not from provenance:
//
//   - emissions, gates and classify transcribe Rust that IS compiled to wasm,
//     so every rule runner checks them cell-by-cell against the real WASM
//     (`runChecker`, `runGates`, `tallyClass`) — the `pred?` column.
//     Independent derivations (this JS vs that Rust); agreement is real
//     information.
//   - inlineViews and reachability transcribe TypeScript/React that is
//     NOT callable headlessly (`filterErrorList`; the input disable; the
//     blank-marker reducer clearing). They are PREDICTIONS ONLY in a Node
//     runner — there is no independent Node oracle — and are validated
//     against the real DOM only where a browser runner covers the cell
//     (`dom-validate.mjs`). Never let a Node runner manufacture an "observed"
//     inline/reachability from this module and compare it to itself: that
//     check would be tautological.
//
// Provenance (so each function can be re-audited):
//   - emissions → sequent-core `ballot_codec/checker.rs`, called from
//     `raw_ballot.rs::decode` in the order invalid → over → min → under →
//     blank → preference-gaps → duplicated-rank. Not modelled: the
//     config-sanity and encoding-error emissions
//     (`check_max_min_votes_policy`, write-in overflow) — no runner
//     exercises them; see EXPLICIT_OR_ENCODING below.
//   - hardGate  → sequent-core `voting_screen.rs::check_voting_not_allowed_next_util`
//   - softGate  → sequent-core `voting_screen.rs::check_voting_error_dialog_util`
//   - classify  → velvet-core `extended_metrics.rs::classify_ballot`
//   - inlineViews → voting-portal `InvalidErrorsList.tsx::filterErrorList`
//     (the alert-visibility rules, the untouched-clear, the dedup block —
//     Defect 3's self-match included — and the master keep-list)
//   - reachability → voting-portal `Question.tsx`/`Answer.tsx` (the DISABLE
//     over-vote policy disables further inputs at max) and
//     `ballotSelectionsSlice.ts::setBallotSelectionBlankVote` (selecting the
//     blank marker clears co-selected regulars)

/** Message keys — single source (were string literals scattered per runner). */
export const MSG = {
    selectedMax: "errors.implicit.selectedMax",
    selectedMin: "errors.implicit.selectedMin",
    blankVote: "errors.implicit.blankVote",
    underVote: "errors.implicit.underVote",
    overVoteDisabled: "errors.implicit.overVoteDisabled",
    duplicatedPosition: "errors.implicit.duplicatedPosition",
    preferenceOrderWithGaps: "errors.implicit.preferenceOrderWithGaps",
    explicitNotAllowed: "errors.explicit.notAllowed",
    explicitAlert: "errors.explicit.alert",
}

/** Messages whose `error_type` is Explicit or EncodingError, tripping the
 *  hard gate's fast path (`invalid_errors.any(Explicit || EncodingError)` in
 *  `check_voting_not_allowed_next_util`). The encoding-error keys are not
 *  produced by `emissions` (no runner exercises them); they are listed for
 *  faithfulness to the gate, so a future runner that emits one gets the
 *  right gate. */
const EXPLICIT_OR_ENCODING = new Set([
    MSG.explicitNotAllowed,
    "errors.configuration.multipleExplicitInvalidCandidates",
    "errors.configuration.multipleExplicitBlankCandidates",
    "errors.encoding.invalidMinVotes",
    "errors.encoding.invalidMaxVotes",
])

/** Baseline policy values (a contest's `presentation` defaults, per
 *  FIXTURE_VARIANCE.md §10.A). A caller passes only the policies it varies;
 *  the rest resolve here. The clauses these govern are inert for a rule
 *  runner whose cells never satisfy them (wrong error identity, or
 *  selections/max out of range) — which is what makes composing the FULL
 *  behaviour neutral against each runner's isolated grid (the cells that
 *  vary only its own rule's dimensions, others at these baselines). */
export const DEFAULTS = {
    invalid: "allowed",
    blank: "allowed",
    over: "allowed-with-msg-and-alert",
    under: "allowed",
    dup: "allowed-warn-and-dialog",
    gap: "allowed-warn-and-dialog",
}

/**
 * VoteState — what the voter did, independent of wire encoding. All fields
 * except `regulars` default to false.
 * @typedef {Object} VoteState
 * @property {number}  regulars        selected regular (non-marker) candidates.
 *   For preferential contests this is the number of candidates at rank 1
 *   (`selected === 0`) — the count the gates use.
 * @property {boolean} [blankMarker]     the explicit-blank marker candidate is
 *   selected
 * @property {boolean} [explicitInvalid] the ballot is explicitly invalid — the
 *   `is_explicit_invalid` flag is set, whether via the null-vote marker or
 *   directly (the two routes converge; recorded in invalid-rule.md)
 * @property {boolean} [decline]         the ballot-level decline-to-vote bit
 *   (multi-contest encodings only; single-contest decode hardcodes false)
 * @property {boolean} [duplicateRanks]  two candidates share a rank
 *   (preferential only)
 * @property {boolean} [rankGaps]        the ranking skips a rank
 *   (preferential only)
 *
 * Config — the contest knobs the mapping reads.
 * @typedef {Object} Config
 * @property {number} min                contest `min_votes`
 * @property {number} max                contest `max_votes`
 * @property {Partial<typeof DEFAULTS>} [policies]  varied policies
 */

/** The marker-inclusive selection count (`num_selected_with_markers` in
 *  `raw_ballot.rs`, `selections_with_markers` in `voting_screen.rs`): a
 *  selected blank marker and a set explicit-invalid flag each count as one
 *  selection. The min/over/under/blank rules all compare against this count,
 *  not the regular-candidate count. */
export function selectionsWithMarkers(vs) {
    return (vs.regulars ?? 0) + (vs.blankMarker ? 1 : 0) + (vs.explicitInvalid ? 1 : 0)
}

/** The ballot-shape class the tally classifier reads. `marker`/`mixed`
 *  concern the explicit-BLANK marker only (`classify_ballot` collects
 *  explicit-blank candidate ids; the invalid marker is represented by the
 *  flag, which short-circuits earlier in the precedence). */
export function selectionClass(vs) {
    if (vs.blankMarker) return vs.regulars > 0 ? "mixed" : "marker"
    return vs.regulars > 0 ? "regular" : "none"
}

/**
 * The checker stage — which errors/alerts `checker.rs` records for this
 * (config × vote-state). Push order mirrors the decode call order in
 * `raw_ballot.rs` (invalid → over → min → under → blank → gaps → dup).
 * @param {Config} config
 * @param {VoteState} vs
 * @returns {{errors: string[], alerts: string[]}}
 */
export function emissions(config, vs) {
    const p = {...DEFAULTS, ...(config.policies ?? {})}
    const n = selectionsWithMarkers(vs)
    const min = config.min ?? 0
    const max = config.max ?? 1
    const errors = []
    const alerts = []
    // invalid rule — fires only on an explicitly-invalid ballot
    if (vs.explicitInvalid) {
        if (p.invalid === "not-allowed") errors.push(MSG.explicitNotAllowed)
        if (p.invalid === "warn-invalid-implicit-and-explicit")
            alerts.push(MSG.explicitAlert)
    }
    // over-vote rule — the error is unconditional; the policy governs only the
    // alert (and, at exactly max under DISABLE, the "maximum reached" hint)
    if (n > max) {
        errors.push(MSG.selectedMax)
        if (p.over !== "allowed") alerts.push(MSG.selectedMax)
    } else if (n === max && p.over === "not-allowed-with-msg-and-disable") {
        alerts.push(MSG.overVoteDisabled)
    }
    // min-vote rule — a fixed rule with no policy of its own: always an error
    if (n < min) errors.push(MSG.selectedMin)
    // under-vote rule — alerts only, never errors; its zone `min ≤ n < max`
    // includes n = 0 when min is 0 (the S4 overlap with the blank rule)
    if (n >= min && n < max && p.under !== "allowed") alerts.push(MSG.underVote)
    // blank rule — skipped entirely for an explicitly-invalid ballot
    if (n === 0 && !vs.explicitInvalid && p.blank !== "allowed") {
        ;(p.blank === "not-allowed" ? errors : alerts).push(MSG.blankVote)
    }
    // preferential rules — both policy variants emit identically; the policy
    // decides only which gate reacts
    if (vs.rankGaps) errors.push(MSG.preferenceOrderWithGaps)
    if (vs.duplicateRanks) errors.push(MSG.duplicatedPosition)
    return {errors, alerts}
}

/**
 * A cell's signal facts — the input record of the downstream functions
 * (gates, filter). `errors`/`alerts` come from `emissions` (or, for a
 * derived view over a recording, from the observed checker output).
 * @typedef {Object} Facts
 * @property {string[]} errors            checker `invalid_errors` message keys
 * @property {string[]} alerts            checker `invalid_alerts` message keys
 * @property {boolean}  explicitInvalid   the `is_explicit_invalid` flag
 * @property {number}   selections        `selections_with_markers`
 * @property {number}   min               contest `min_votes`
 * @property {number}   max               contest `max_votes`
 * @property {"none"|"regular"|"marker"|"mixed"} selection  ballot shape (classify)
 * @property {Partial<typeof DEFAULTS>} policies            varied policies
 */

/** Hard gate — `check_voting_not_allowed_next_util`. True ⇒ a blocking
 *  dialog: the voter cannot reach review until the ballot is fixed. */
export function hardGate(f) {
    const p = {...DEFAULTS, ...(f.policies ?? {})}
    const errors = f.errors ?? []
    const selections = f.selections ?? 0
    const max = f.max ?? 1
    return (
        errors.some((m) => EXPLICIT_OR_ENCODING.has(m)) ||
        (errors.length > 0 && p.invalid === "not-allowed") ||
        (selections === 0 && p.blank === "not-allowed") ||
        (selections > max && p.over === "not-allowed-with-msg-and-alert") ||
        (p.dup === "not-allowed-warn-and-dialog" &&
            errors.includes(MSG.duplicatedPosition)) ||
        (p.gap === "not-allowed-warn-and-dialog" &&
            errors.includes(MSG.preferenceOrderWithGaps))
    )
}

/** Soft gate — `check_voting_error_dialog_util`. True ⇒ a dismissible
 *  dialog: the voter is warned but may continue. */
export function softGate(f) {
    const p = {...DEFAULTS, ...(f.policies ?? {})}
    const errors = f.errors ?? []
    const selections = f.selections ?? 0
    const min = f.min ?? 0
    const max = f.max ?? 1
    return (
        (errors.length > 0 && p.invalid !== "allowed") ||
        (p.invalid === "warn-invalid-implicit-and-explicit" &&
            !!f.explicitInvalid) ||
        (p.blank === "warn" && selections === 0) ||
        (selections > max && p.over === "allowed-with-msg-and-alert") ||
        (selections > 0 &&
            selections >= min &&
            selections < max &&
            p.under === "warn-and-alert") ||
        (p.dup === "allowed-warn-and-dialog" &&
            errors.includes(MSG.duplicatedPosition)) ||
        (p.gap === "allowed-warn-and-dialog" &&
            errors.includes(MSG.preferenceOrderWithGaps))
    )
}

/** Tally classifier — `classify_ballot`. First matching guard wins; see the
 *  precedence note in `classifier-table.md`. `invalid = flag || errors`. */
export function classify(f) {
    const invalid = !!f.explicitInvalid || !!f.hasErrors
    const nothingSelected = f.selection === "none"
    const blank = !invalid && nothingSelected
    if (f.decline) return blank ? "Declined" : "ImplicitInvalid"
    if (invalid) return f.explicitInvalid ? "ExplicitInvalid" : "ImplicitInvalid"
    if (f.selection === "mixed") return "ImplicitInvalid"
    if (f.selection === "marker") return "ExplicitBlank"
    if (nothingSelected) return "ImplicitBlank"
    return "Valid"
}

/** One observation point of the booth message filter — the WarnBox message
 *  keys `filterErrorList` leaves rendered, transcribed step-by-step in the
 *  production order (alert visibility → dedup → the master keep-list on
 *  errors; errors render before alerts). Internal: callers use `inlineViews`.
 */
function renderedKeys(f, isReview) {
    const p = {...DEFAULTS, ...(f.policies ?? {})}
    const errors = f.errors ?? []
    // Alert visibility — the only point-dependent rules: WARN_ONLY_IN_REVIEW
    // alerts are hidden during voting, and the `overVoteDisabled` "maximum
    // reached" hint is hidden at review.
    let alerts = (f.alerts ?? []).filter(
        (m) =>
            !(
                (m === MSG.underVote &&
                    !isReview &&
                    p.under === "warn-only-in-review") ||
                (m === MSG.blankVote &&
                    !isReview &&
                    p.blank === "warn-only-in-review") ||
                (m === MSG.overVoteDisabled && isReview)
            )
    )
    // Dedup — production's `containsError` searches the visibility-filtered
    // alerts plus the RAW (pre-keep-list) errors. The selectedMax predicate
    // matches the very alert under examination, so a selectedMax alert is
    // always dropped (Defect 3 in UPSTREAM_FINDINGS.md — transcribed as-is).
    const present = (m) => alerts.includes(m) || errors.includes(m)
    alerts = alerts.filter(
        (m) =>
            !(
                (m === MSG.underVote && present(MSG.blankVote)) ||
                (m === MSG.selectedMax && present(MSG.selectedMax))
            )
    )
    // The master keep-list: under `invalid = allowed` every error is hidden
    // except the two carve-outs.
    const keptErrors = errors.filter((m) => {
        if (p.invalid !== "allowed") return true
        if (m === MSG.selectedMax && p.over !== "allowed") return true
        if (m === MSG.blankVote && p.blank === "not-allowed") return true
        return false
    })
    return [...keptErrors, ...alerts]
}

/** Booth message filter — `filterErrorList`, at every observation point.
 *  The observation point is an index of this surface's OUTPUT, not an input
 *  of the whole mapping: only the inline surface varies with when/where the
 *  voter looks. Three points:
 *    - votingUntouched — the voting screen before the contest has any
 *      selection (`isTouched`, Question.tsx state, armed by
 *      `choices.some(selected > -1)` — note the null-vote marker alone never
 *      arms it): the untouched-clear empties both lists, so this view is
 *      constantly empty.
 *    - voting  — the voting screen once touched.
 *    - review  — the review screen (touch is assumed there).
 *
 *  PREDICTION ONLY in a Node runner (see the module header): validate against
 *  the real DOM in a browser runner, never against a re-computation of this. */
export function inlineViews(f) {
    return {
        votingUntouched: [],
        voting: renderedKeys(f, false),
        review: renderedKeys(f, true),
    }
}

/**
 * Reachability — can the booth UI form this state at all? Two prevention
 * mechanisms exist, each returning its own value so a browser runner can
 * verify the mechanism, not just the absence:
 *   - "inputs_disabled" — under the DISABLE over-vote policy the (max+1)th
 *     control is disabled, so a past-max state cannot be clicked together
 *     (`Question.tsx`/`Answer.tsx`).
 *   - "marker_cleared" — selecting the blank marker clears co-selected
 *     regulars (and vice versa), so {blank marker + regular} collapses
 *     (`setBallotSelectionBlankVote`). The INVALID marker deliberately does
 *     not clear — {regular + null marker} forms; that asymmetry is finding
 *     S5 (UPSTREAM_FINDINGS.md).
 * Prevention prunes which states the booth can produce; it does not change
 * the mapping over states that exist anyway (hand-built or decoded records
 * still flow through the checkers — defense-in-depth), which is why `f`
 * stays total and reachability is reported alongside the effects rather
 * than folded into them.
 *
 * PREDICTION ONLY in a Node runner (see the module header).
 * @param {Config} config
 * @param {VoteState} vs
 * @returns {"yes" | "inputs_disabled" | "marker_cleared"}
 */
export function reachability(config, vs) {
    const p = {...DEFAULTS, ...(config.policies ?? {})}
    const max = config.max ?? 1
    if (
        p.over === "not-allowed-with-msg-and-disable" &&
        selectionsWithMarkers(vs) > max
    ) {
        return "inputs_disabled"
    }
    if (vs.blankMarker && (vs.regulars ?? 0) > 0) return "marker_cleared"
    return "yes"
}

/**
 * The complete mapping — one call per characterization cell.
 *
 * Of the six output components, three are the effect surfaces (`inline`,
 * `gate`, `tally` — VALIDATION_LOGIC_DISTILLATION.md §1, "The surfaces,
 * enumerated"); the others are their companions: `emissions` is the checker
 * record the surfaces consume (never voter-perceived; WASM-checkable),
 * `dialog` is the gate surface's voter-facing projection, and
 * `reachability` is a domain property, not an effect.
 *
 * (config × voteState) determines everything; there is no observation-
 * context input. The observation point exists only inside the OUTPUT, as
 * the index of the `inline` component (`votingUntouched` / `voting` /
 * `review`) — the one surface whose content varies with when and where the
 * voter looks. The gate pair is consulted at a single fixed moment (the
 * Next/review transition); the tally class is observed after casting,
 * outside the booth's timeline (and never per-ballot by the voter — only
 * through result aggregates); reachability is a property of interaction
 * attempts, not an effect. An earlier revision took an
 * `observation_context` argument — wrong shape: it quantified a per-surface
 * index over the whole product.
 *
 * @param {Config} config
 * @param {VoteState} vs
 * @returns {{
 *   emissions: {errors: string[], alerts: string[]},
 *   inline: {votingUntouched: string[], voting: string[], review: string[]},
 *   gate: {hard: boolean, soft: boolean},
 *   dialog: "none" | "dismissible" | "blocking",
 *   reachability: "yes" | "inputs_disabled" | "marker_cleared",
 *   tally: string,
 * }}
 */
export function f(config, vs) {
    const em = emissions(config, vs)
    const facts = {
        errors: em.errors,
        alerts: em.alerts,
        explicitInvalid: !!vs.explicitInvalid,
        selections: selectionsWithMarkers(vs),
        min: config.min ?? 0,
        max: config.max ?? 1,
        selection: selectionClass(vs),
        policies: config.policies,
    }
    const hard = hardGate(facts)
    const soft = softGate(facts)
    return {
        // The checker record — internal (never voter-perceived), exposed
        // because it is independently checkable against the real WASM.
        emissions: em,
        inline: inlineViews(facts),
        gate: {hard, soft},
        // What the voter meets on clicking Next: the hard gate's dialog has
        // no Continue button, the soft gate's is dismissible; hard wins when
        // both fire (`VotingScreen.tsx`, `encryptAndReview`).
        dialog: hard ? "blocking" : soft ? "dismissible" : "none",
        reachability: reachability(config, vs),
        tally: classify({
            decline: !!vs.decline,
            explicitInvalid: !!vs.explicitInvalid,
            hasErrors: em.errors.length > 0,
            selection: selectionClass(vs),
        }),
    }
}
