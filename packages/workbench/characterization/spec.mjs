// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The shared vote-validation specification — the single executable statement
// of the mapping
//
//     f(config, voteState, context) → effects
//
// (VALIDATION_LOGIC_DISTILLATION.md §3): given a contest's configuration and
// a description of what the voter did, which checker messages exist, whether
// the review-transition gates fire, what the voter sees inline, whether the
// booth UI lets the state form at all, and how the tally classifies the
// ballot. `context` names the observation point — the moment/screen at which
// the inline surface is read (see `f`).
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
//   - inlineVisible and reachability transcribe TypeScript/React that is
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
//   - inlineVisible → voting-portal `InvalidErrorsList.tsx::filterErrorList`
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
 *  behaviour neutral against each runner's isolated grid. */
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

/** Booth message filter — `filterErrorList`'s keep-list, at the **review**
 *  surface (the decisive last screen before cast, and the surface the DOM
 *  validators observe). Under `invalid = allowed` every error is hidden except
 *  the two keep-list carve-outs. Two observation-point rules are review-
 *  specific and modelled here: the untouched-clear (empty contests show
 *  nothing while untouched) is a *voting-only* behaviour and does NOT apply at
 *  review, so it is correctly absent; and `overVoteDisabled` (the "maximum
 *  reached" hint) is a voting-only hint that filterErrorList hides at review
 *  (`… && isReview`), so it is excluded here. (WARN_ONLY_IN_REVIEW alerts show
 *  at review; not exercised by the rules with a complete table yet.)
 *
 *  PREDICTION ONLY in a Node runner (see the module header): validate against
 *  the real DOM in a browser runner, never against a re-computation of this. */
export function inlineVisible(f) {
    const p = {...DEFAULTS, ...(f.policies ?? {})}
    const errors = f.errors ?? []
    const alerts = f.alerts ?? []
    const keptErrors = errors.filter((m) => {
        if (p.invalid !== "allowed") return true
        if (m === MSG.selectedMax && p.over !== "allowed") return true
        if (m === MSG.blankVote && p.blank === "not-allowed") return true
        return false
    })
    // `overVoteDisabled` is a voting-only hint, hidden at review.
    const shownAlerts = alerts.filter((m) => m !== MSG.overVoteDisabled)
    return [...keptErrors, ...shownAlerts]
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
 * `context.point` names the observation point of the `inline` component:
 * the screen at which the voter's inline warnings are read. Only "review"
 * (the last screen before cast) is modelled so far; the during-voting
 * surface (the touch gate, WARN_ONLY_IN_REVIEW hiding, the voting-time
 * `overVoteDisabled` hint) is an open gap tracked in
 * characterization/README.md's completeness table. Gates and the dialog are
 * transition-time by definition; emissions, tally and reachability do not
 * depend on the observation point.
 *
 * @param {Config} config
 * @param {VoteState} vs
 * @param {{point: "review"}} [context]
 */
export function f(config, vs, context = {point: "review"}) {
    if (context.point !== "review") {
        throw new Error(
            `observation point "${context.point}" is not modelled yet (only "review")`
        )
    }
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
        errors: em.errors,
        alerts: em.alerts,
        hard,
        soft,
        // What the voter meets on clicking Next: the hard gate's dialog has
        // no Continue button, the soft gate's is dismissible; hard wins when
        // both fire (`VotingScreen.tsx`, `encryptAndReview`).
        dialog: hard ? "blocking" : soft ? "dismissible" : "none",
        inline: inlineVisible(facts),
        reachability: reachability(config, vs),
        tally: classify({
            decline: !!vs.decline,
            explicitInvalid: !!vs.explicitInvalid,
            hasErrors: em.errors.length > 0,
            selection: selectionClass(vs),
        }),
    }
}
