// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Synthetic reconciliation rounds: the diff envelope the generate task
// uploads, and the tasks the wizard polls.
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {storyId} from "@/__stories__/fixtures"
import {ESyncChangeCategory} from "../types"

export const UPLOAD_URL = "https://s3.admin-story.invalid/uploads/reconciliation.csv"
export const FILE_ID = storyId(6, 1)
export const DIFF_ID = storyId(6, 2)
export const PATCH_ID = storyId(6, 3)
export const GENERATE_TASK_ID = storyId(6, 4)
export const APPLY_TASK_ID = storyId(6, 5)

interface DiffItem {
    voter_username: string
    target: "datafix" | "sequent"
    category: ESyncChangeCategory
    field: Record<string, unknown> | null
    failure_reason?: string
}

export const votedOnPaper: DiffItem = {
    voter_username: "voter-001",
    target: "sequent",
    category: ESyncChangeCategory.VOTED_OTHER_CHANNEL,
    field: {KeycloakUA: [{"voted-channel": "NONE"}, {"voted-channel": "PAPER"}]},
}

export const movedWard: DiffItem = {
    voter_username: "voter-002",
    target: "sequent",
    category: ESyncChangeCategory.PROFILE_UPDATE,
    field: {Ward: ["Ward 1", "Ward 2"]},
}

export const votedOnline: DiffItem = {
    voter_username: "voter-003",
    target: "datafix",
    category: ESyncChangeCategory.VOTED_INTERNET,
    field: {Channel: ["NONE", "INTERNET"]},
}

export const countyMismatch: DiffItem = {
    voter_username: "voter-004",
    target: "sequent",
    category: ESyncChangeCategory.ROW_FAILURE,
    field: null,
    failure_reason: "County does not match the municipality",
}

/** The kinds of round a dropped file can produce. */
export enum ERound {
    /** The external system still has to apply its patch. */
    EXTERNAL_PENDING = "externalPending",
    /** Only Sequent-side changes remain. */
    SEQUENT_ONLY = "sequentOnly",
    /** Nothing differs. */
    IN_SYNC = "inSync",
    /** An already applied Sequence dropped again, still showing differences. */
    CONVERGENCE_CHECK = "convergenceCheck",
    /** Some rows cannot be reconciled. */
    ROW_FAILURES = "rowFailures",
}

export function envelope(round: ERound) {
    const items = {
        [ERound.EXTERNAL_PENDING]: [votedOnline, votedOnPaper],
        [ERound.SEQUENT_ONLY]: [votedOnPaper, movedWard],
        [ERound.IN_SYNC]: [],
        [ERound.CONVERGENCE_CHECK]: [movedWard],
        [ERound.ROW_FAILURES]: [movedWard, countyMismatch],
    }[round]
    return {
        sequence: 42,
        generated_at: 0,
        apply_allowed: round !== ERound.CONVERGENCE_CHECK,
        external_patch_document_id: round === ERound.EXTERNAL_PENDING ? PATCH_ID : null,
        items,
    }
}

export function task(
    id: string,
    type: string,
    execution_status: ETaskExecutionStatus,
    annotations: Record<string, unknown> = {}
) {
    return {
        id,
        election_event_id: EVENT_ID,
        tenant_id: TENANT_ID,
        execution_status,
        type,
        start_at: "2026-01-01T00:00:00.000Z",
        end_at: null,
        logs: [],
        annotations,
        executed_by_user: "admin",
    }
}
