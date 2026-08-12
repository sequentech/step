// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The shared vote-validation specification.
//
// Each rule runner's `predict()` supplies only its rule-specific CHECKER
// EMISSIONS — which `invalid_errors` / `invalid_alerts` the checker produces
// for a given (config × state). Everything downstream — the two review-
// transition gates, the tally classifier, the booth message filter (inline
// visibility), and the input-constraint model — composes HERE, as one
// transcription of the production rules rather than seven partial copies.
//
// This is the "single artifact" of VALIDATION_LOGIC_DISTILLATION.md §5.1,
// taking its first concrete form: the embryonic specification. It is a
// transcription of production TypeScript/Rust and therefore fallible — its
// authority comes from being CHECKED, not from provenance:
//
//   - gates and classify are transcriptions of Rust that IS compiled to WASM,
//     so every rule runner checks them cell-by-cell against the real WASM
//     (`runGates`, `tallyClass`) — the `pred?` column. Independent
//     derivations (this JS vs that Rust); agreement is real information.
//   - inlineVisible and inputConstraint transcribe TypeScript/React that is
//     NOT callable headlessly (`filterErrorList`; the Question/Answer input
//     disable). They are PREDICTIONS ONLY in a Node runner — there is no
//     independent Node oracle — and are validated against the real DOM only
//     where a browser runner covers the cell. Never let a Node runner
//     manufacture an "observed" inline/constraint from this module and
//     compare it to itself: that check would be tautological.
//
// Provenance (so each function can be re-audited):
//   - hardGate  → sequent-core `voting_screen.rs::check_voting_not_allowed_next_util`
//   - softGate  → sequent-core `voting_screen.rs::check_voting_error_dialog_util`
//   - classify  → velvet-core `extended_metrics.rs::classify_ballot`
//   - inlineVisible → voting-portal `InvalidErrorsList.tsx::filterErrorList`
//   - inputConstraint → voting-portal `Question.tsx`/`Answer.tsx` (disable)

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
 *  produced by the current rule runners; they are listed for faithfulness to
 *  the gate, so a future runner that emits one gets the right gate. */
const EXPLICIT_OR_ENCODING = new Set([
    MSG.explicitNotAllowed,
    "errors.configuration.multipleExplicitInvalidCandidates",
    "errors.configuration.multipleExplicitBlankCandidates",
    "errors.encoding.invalidMinVotes",
    "errors.encoding.invalidMaxVotes",
])

/** Baseline policy values (a contest's `presentation` defaults, per
 *  FIXTURE_VARIANCE.md §10.A). A rule runner passes only the policies it
 *  varies; the rest resolve here. The clauses these govern are inert for a
 *  runner whose cells never satisfy them (wrong error identity, or
 *  selections/max out of range) — which is what makes composing the FULL
 *  gate behaviour-neutral against each runner's partial hand-copy. */
export const DEFAULTS = {
    invalid: "allowed",
    blank: "allowed",
    over: "allowed-with-msg-and-alert",
    under: "allowed",
    dup: "allowed-warn-and-dialog",
    gap: "allowed-warn-and-dialog",
}

/**
 * A cell's signal facts. Rule-specific fields (`errors`, `alerts`,
 * `explicitInvalid`) come from the runner's checker emissions; the rest
 * describe the ballot shape and configuration.
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

/** Booth message filter — `filterErrorList`'s keep-list. Given the checker
 *  emissions, returns what stays VISIBLE inline to the voter. Under
 *  `invalid = allowed` every error is hidden except the two keep-list
 *  carve-outs; alerts are not suppressed by the invalid policy (their own
 *  observation-point filtering — WARN_ONLY_IN_REVIEW, untouched-clear — is a
 *  browser concern this during-voting view does not model).
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
    return [...keptErrors, ...alerts]
}

/** Input-constraint model — the surface only a browser can observe directly
 *  (`Question`/`Answer` disable the (max+1)th control under the DISABLE
 *  over-vote policy). Returns `"inputs_disabled"` when the state requires
 *  selecting past a disabled control — i.e. the state is UNREACHABLE through
 *  the real UI — else `null`.
 *
 *  PREDICTION ONLY in a Node runner (see the module header). */
export function inputConstraint(f) {
    const p = {...DEFAULTS, ...(f.policies ?? {})}
    const selections = f.selections ?? 0
    const max = f.max ?? 1
    if (p.over === "not-allowed-with-msg-and-disable" && selections > max) {
        return "inputs_disabled"
    }
    return null
}
