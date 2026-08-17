// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Shared harness for validation-behaviour characterization (see
// docs/VALIDATION_LOGIC_DISTILLATION.md §5.3 step 1).
//
// Loads the sequent-core wasm package in Node — no browser, no dev server —
// and exposes the three Rust entry points a characterization cell needs:
//
//   - runChecker(decodedContest, ballotEml): the encode→decode round-trip
//     (`test_contest_reencoding_js`), i.e. exactly what the booth runs per
//     click. Returns the re-decoded contest with invalid_errors /
//     invalid_alerts populated by checker.rs.
//   - runGates(contests, decodedRecord): both submission gates
//     (`check_voting_not_allowed_next` / `check_voting_error_dialog`).
//
// The TypeScript filter layer cannot run headlessly —
// `filterErrorList` is component-internal — and is validated separately
// in the browser (dom-validate.mjs and the dependency pipeline).

import {readFileSync} from "node:fs"
import {fileURLToPath, pathToFileURL} from "node:url"
import path from "node:path"

const here = path.dirname(fileURLToPath(import.meta.url))
const PKG = path.resolve(here, "../../sequent-core/pkg")

let mod = null

export async function loadWasm() {
    if (mod) return mod
    mod = await import(pathToFileURL(path.join(PKG, "index.js")).href)
    const bytes = readFileSync(path.join(PKG, "index_bg.wasm"))
    try {
        mod.initSync({module: bytes})
    } catch {
        await mod.default(bytes)
    }
    return mod
}

/** Encode→decode round-trip through checker.rs — the checker emissions. */
export function runChecker(decodedContest, ballotEml) {
    return mod.test_contest_reencoding_js(decodedContest, ballotEml)
}

/** Submission gates over already-decoded contests — the gate pair. */
export function runGates(contests, decodedRecord) {
    return {
        hard: mod.check_voting_not_allowed_next(contests, decodedRecord),
        soft: mod.check_voting_error_dialog(contests, decodedRecord),
    }
}

/** The bundled fixture that carries marker candidates (FIXTURE_VARIANCE §13.2). */
export function loadMarkerFixture() {
    const p = path.resolve(
        here,
        "../app/src/fixtures/snapshots/explicit-blank-invalid.json"
    )
    return JSON.parse(readFileSync(p, "utf8"))
}

export function extractErrors(decoded) {
    return {
        errors: (decoded.invalid_errors ?? []).map((e) => e.message),
        alerts: (decoded.invalid_alerts ?? []).map((e) => e.message),
    }
}

// ---------------------------------------------------------------------------
// Tally side: velvet-wasm, loaded the same way. `tallyClass` runs a
// single decoded ballot through the real tally and reads which counter
// incremented — that counter IS the ballot's BallotClass, recorded rather
// than predicted.
// ---------------------------------------------------------------------------
const VELVET_PKG = path.resolve(here, "../velvet-wasm/pkg")
let velvet = null

export async function loadVelvetWasm() {
    if (velvet) return velvet
    velvet = await import(pathToFileURL(path.join(VELVET_PKG, "velvet_wasm.js")).href)
    const bytes = readFileSync(path.join(VELVET_PKG, "velvet_wasm_bg.wasm"))
    try {
        velvet.initSync({module: bytes})
    } catch {
        await velvet.default(bytes)
    }
    return velvet
}

/**
 * The no-silent-discount predicate — THE single definition, shared by the
 * rule runners (for the derived ⚠ column) and by no-silent-discount.mjs
 * (for the property query), so the two can never drift.
 *
 * Derived, not observed (convention 3): it joins the checker/filter/gate
 * observables with the tally class. True = the voter gets no booth signal
 * at any casting point, yet the tally discards the ballot as ImplicitInvalid.
 */
export function isSilentDiscount(cell) {
    const o = cell.observed
    const d = cell.derived_inline ?? {}
    // "No signal" must hold at every observation point of the inline effect the voter
    // passes through — the touched voting screen and the review screen (the
    // untouched view is constantly empty and adds nothing).
    const inlineShown =
        (d.voting ?? []).length > 0 || (d.review ?? []).length > 0
    const gate = o.hard || o.soft
    const constrained = o.constraint === "inputs_disabled"
    return (
        o.tally === "ImplicitInvalid" && !inlineShown && !gate && !constrained
    )
}

/** Tally one decoded ballot; return its BallotClass as recorded by the
 *  real counters on ContestResult. */
export function tallyClass(contest, decodedBallot) {
    const out = velvet.tally_decoded_ballots(JSON.stringify(contest), [
        JSON.stringify(decodedBallot),
    ])
    const r = JSON.parse(out)
    const declined = r.extended_metrics?.total_declined_to_vote ?? 0
    if (declined > 0) return "Declined"
    if ((r.invalid_votes?.explicit ?? 0) > 0) return "ExplicitInvalid"
    if ((r.invalid_votes?.implicit ?? 0) > 0) return "ImplicitInvalid"
    if ((r.blank_votes?.explicit ?? 0) > 0) return "ExplicitBlank"
    if ((r.blank_votes?.implicit ?? 0) > 0) return "ImplicitBlank"
    return "Valid"
}
