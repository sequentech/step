// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    packageImportPhase,
    type PackageImportPhase,
    type PackageImportSignals,
} from "./PackageImportWorkflow"

const readyToUpload: PackageImportSignals = {
    permitsImport: true,
    rosterLoading: false,
    submitting: false,
    running: false,
    canApply: false,
    hasBlockers: false,
    previewRefused: false,
    unreadable: false,
    noRecord: false,
    hasError: false,
    hasScopeMismatch: false,
}

it.each<[PackageImportPhase, Partial<PackageImportSignals>]>([
    ["waitingForTrust", {permitsImport: false, rosterLoading: true}],
    ["trustBlocked", {permitsImport: false, canApply: true}],
    ["choose", {}],
    ["starting", {submitting: true, submittedMode: "apply"}],
    ["starting", {running: true}],
    ["previewing", {running: true, submittedMode: "preview"}],
    ["previewBlocked", {hasBlockers: true, outcome: "previewed"}],
    ["previewBlocked", {previewRefused: true, submittedMode: "preview", outcome: "refused"}],
    ["attention", {previewRefused: true, submittedMode: "apply", outcome: "refused"}],
    ["previewAgain", {outcome: "previewed"}],
    ["previewAgain", {hasScopeMismatch: true}],
    ["review", {outcome: "previewed", canApply: true}],
    ["applying", {running: true, submittedMode: "apply", outcome: "applied"}],
    ["complete", {submittedMode: "apply", outcome: "applied", executionStatus: "SUCCESS"}],
    // An applied record alone is not success: the task row must agree, as for the receipt.
    ["attention", {submittedMode: "apply", outcome: "applied"}],
    ["attention", {submittedMode: "apply", outcome: "applied", executionStatus: "FAILED"}],
    ["attention", {submittedMode: "apply", outcome: "applied", executionStatus: "CANCELLED"}],
    // A refused Preview for requirements no longer selected is stale, not the current blocker.
    [
        "previewAgain",
        {previewRefused: true, previewStale: true, submittedMode: "preview", outcome: "refused"},
    ],
    // Staleness never hides an Apply run's result.
    ["attention", {previewStale: true, submittedMode: "apply", outcome: "failed"}],
    ["choose", {submittedMode: "preview", outcome: "applied"}],
    ["attention", {unreadable: true, running: true, submittedMode: "apply"}],
    ["attention", {noRecord: true}],
    ["attention", {hasError: true}],
    ["attention", {submittedMode: "apply", outcome: "unknown"}],
    ["attention", {submittedMode: "apply", outcome: "refused"}],
    ["attention", {submittedMode: "apply", outcome: "failed"}],
])("shows %s without inferring a different operation", (expected, values) => {
    expect(packageImportPhase({...readyToUpload, ...values})).toBe(expected)
})
