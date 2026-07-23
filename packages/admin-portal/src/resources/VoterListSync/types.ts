// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Shapes used by the external voter registry reconciliation wizard.
 * Wire values match the backend's Rust enums exactly, so these describe
 * GraphQL response shapes directly with no translation layer.
 */

// Source-of-truth categories from the "Handling Inconsistencies" section,
// plus REENABLED (a voter Sequent disabled solely because of a prior
// Datafix delete call is re-enabled when the file no longer reports them
// Deleted).
export enum ESyncChangeCategory {
    VOTED_INTERNET = "VOTED_INTERNET", // A: Sequent holds a valid internet ballot, Datafix says NONE
    VOTED_OTHER_CHANNEL = "VOTED_OTHER_CHANNEL", // B: Datafix reports a non-INTERNET channel
    DISABLED_DELETE_CALL = "DISABLED_DELETE_CALL", // C: Deleted=true in the reconciliation file
    DELETION_REVERTED = "DELETION_REVERTED", // C exception: voter has voted, deletion is not applied
    PROFILE_UPDATE = "PROFILE_UPDATE", // C: Ward/Poll/SchoolSupportCode/DoB changed
    VOTER_ADDED = "VOTER_ADDED", // D: voter missing on one side
    REENABLED = "REENABLED", // re-enabled after a Datafix delete-call disable is undone
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
