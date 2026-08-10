// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Shared harness for validation-behaviour characterization (see
// docs/VALIDATION_LOGIC_DISTILLATION.md §5.3 step 1).
//
// Loads the sequent-core wasm package in Node — no browser, no dev server —
// and exposes the three Rust surfaces a characterization cell needs:
//
//   - runChecker(decodedContest, ballotEml): the encode→decode round-trip
//     (`test_contest_reencoding_js`), i.e. exactly what the booth runs per
//     click. Returns the re-decoded contest with invalid_errors /
//     invalid_alerts populated by checker.rs.
//   - runGates(contests, decodedRecord): both submission gates
//     (`check_voting_not_allowed_next` / `check_voting_error_dialog`).
//
// The third surface (the TypeScript filter layer) cannot run headlessly —
// `filterErrorList` is component-internal — and is characterized separately
// in the browser (blank-rule.browser.mjs).

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

/** Encode→decode round-trip through checker.rs (booth layer 1). */
export function runChecker(decodedContest, ballotEml) {
    return mod.test_contest_reencoding_js(decodedContest, ballotEml)
}

/** Submission gates over already-decoded contests (booth layer 2). */
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
