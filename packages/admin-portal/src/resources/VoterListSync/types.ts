// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Shapes shared by the two Datafix reconciliation wizards (generate patches /
 * apply patch). Mirrors the reconciliation file, patch file and diff-row
 * shapes from DatafixPossibleImplementation.md so the mock layer in
 * mockSyncEngine.ts can be swapped for real GraphQL calls without touching
 * the wizard components or SyncDiffTable.
 */

export enum ESyncFileKind {
    RECONCILIATION = "RECONCILIATION",
    PATCH = "PATCH",
}

// Source-of-truth categories from the "Handling Inconsistencies" section.
export enum ESyncChangeCategory {
    VOTED_INTERNET = "VOTED_INTERNET", // A: Sequent holds a valid internet ballot, Datafix says NONE
    VOTED_OTHER_CHANNEL = "VOTED_OTHER_CHANNEL", // B: Datafix reports a non-INTERNET channel
    DISABLED = "DISABLED", // C: Deleted=true in the reconciliation file
    DELETION_REVERTED = "DELETION_REVERTED", // C exception: voter has voted, deletion is not applied
    PROFILE_UPDATE = "PROFILE_UPDATE", // C: Ward/Poll/SchoolSupportCode/DoB changed
    VOTER_ADDED = "VOTER_ADDED", // D: voter missing on one side
    ROW_FAILURE = "ROW_FAILURE", // CountyMun mismatch or the voted-other-channel guard
}

export type ESyncPatchTarget = "datafix" | "sequent"

export interface SyncFileMeta {
    sequence: number
    generatedAt: number // unix seconds, UTC
}

// Fixed column order for the "Patch Files Format" - every patch row carries
// all of these as <Field>_old/<Field>_new pairs, VoterID is the row key and
// is never one of them (VoterID cannot change).
export const PATCH_FIELDS = [
    "CountyMun",
    "DoB",
    "Ward",
    "Poll",
    "SchoolSupportCode",
    "Channel",
    "Deleted",
] as const

export type PatchField = (typeof PATCH_FIELDS)[number]

export interface PatchFieldValue {
    oldValue: string
    newValue: string
}

/**
 * One patch row: every PATCH_FIELDS entry is always present (unchanged
 * fields carry the same value in both columns), only voters with at least
 * one changed field get a record. This is the source of truth for CSV
 * output; SyncDiffRow (below) is a per-changed-field flattening of it for
 * display.
 */
export interface SyncVoterRecord {
    voterId: string
    target: ESyncPatchTarget
    category: ESyncChangeCategory
    fields: Record<PatchField, PatchFieldValue>
}

export interface ParsedReconciliationRow {
    countyMun: string
    voterId: string
    dob: string
    ward: string
    poll: string
    schoolSupportCode: string
    channel: string
    deleted: boolean
}

export interface SyncDiffRow {
    id: string
    voterId: string
    field: string
    label: string
    oldValue: string
    newValue: string
    category: ESyncChangeCategory
    target: ESyncPatchTarget
    /** Set for ROW_FAILURE rows: why the row was excluded from the patch. */
    failureReason?: string
}

export interface RowFailure {
    voterId: string
    reason: string
}

export interface SyncTaskResult {
    rowFailures: RowFailure[]
}
