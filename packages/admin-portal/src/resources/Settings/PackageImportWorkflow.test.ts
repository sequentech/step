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
    ["complete", {submittedMode: "apply", outcome: "applied"}],
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
