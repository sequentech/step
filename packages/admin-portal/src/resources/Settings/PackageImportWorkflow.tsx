// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ApplyMode, Outcome} from "./configurationPackage"

export type PackageImportPhase =
    | "waitingForTrust"
    | "trustBlocked"
    | "choose"
    | "starting"
    | "previewing"
    | "previewBlocked"
    | "previewAgain"
    | "review"
    | "applying"
    | "complete"
    | "attention"

export interface PackageImportSignals {
    permitsImport: boolean
    rosterLoading: boolean
    submitting: boolean
    running: boolean
    submittedMode?: ApplyMode
    outcome?: Outcome
    canApply: boolean
    hasBlockers: boolean
    previewRefused: boolean
    unreadable: boolean
    noRecord: boolean
    hasError: boolean
    hasScopeMismatch: boolean
}

// Presentation only: the phase picks copy and emphasis. It admits nothing; the parent keeps
// its own gates (canApply, trust, confirmation) and must not feed a phase back into them.
export const packageImportPhase = (signals: PackageImportSignals): PackageImportPhase => {
    const {submittedMode, outcome} = signals

    // Missing trust outranks everything else on the page.
    if (!signals.permitsImport) {
        return signals.rosterLoading ? "waitingForTrust" : "trustBlocked"
    }
    // A request in flight has no task yet, so an earlier run's mode says nothing about it.
    if (signals.submitting) {
        return "starting"
    }
    if (signals.unreadable || signals.noRecord || signals.hasError) {
        return "attention"
    }
    if (signals.running) {
        if (!submittedMode) {
            return "starting"
        }
        return submittedMode === "apply" ? "applying" : "previewing"
    }
    if (signals.previewRefused && submittedMode === "preview") {
        return "previewBlocked"
    }
    if (outcome === "failed" || outcome === "refused" || outcome === "unknown") {
        return "attention"
    }
    if (outcome === "applied" && submittedMode === "apply") {
        return "complete"
    }
    if (signals.canApply) {
        return "review"
    }
    if (signals.hasBlockers) {
        return "previewBlocked"
    }
    // A preview that finished but may not be applied is no longer the selected one:
    // the options, the checksum or the receiver's trust changed since.
    if (signals.hasScopeMismatch || outcome === "previewed") {
        return "previewAgain"
    }
    return "choose"
}
