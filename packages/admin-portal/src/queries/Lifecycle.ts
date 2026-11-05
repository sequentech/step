// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"
import type {
    ILifecycleSnapshot,
    IRetainedSignedClose,
    IScheduleImportPreview,
    IScheduledOutcomeChange,
    IScheduledOutcomeRow,
} from "@/types/lifecycle"

// The VOTE-LIFECYCLE Hasura actions (design §6). Their outputs are typed here
// until the generated GraphQL types include them.

export const GET_SCHEDULED_OUTCOMES = gql`
    query GetScheduledOutcomes($electionEventId: uuid!) {
        get_scheduled_outcomes(election_event_id: $electionEventId) {
            outcomes
            retained_closes
        }
    }
`
export interface GetScheduledOutcomesData {
    get_scheduled_outcomes?: {
        outcomes: Array<IScheduledOutcomeRow>
        retained_closes?: Array<IRetainedSignedClose> | null
    } | null
}

export const PREVIEW_SCHEDULED_OUTCOME_CHANGE = gql`
    query PreviewScheduledOutcomeChange($electionEventId: uuid!, $change: jsonb!) {
        preview_scheduled_outcome_change(election_event_id: $electionEventId, change: $change) {
            applies
            applies_message_key
            changes
        }
    }
`
/** How a rule or policy change applies (`tightens`, `loosens`, `tightens-and-loosens`). */
export interface IChangeApplies {
    applies?: string | null
    applies_message_key?: string | null
    changes: Array<IScheduledOutcomeChange>
}
export interface PreviewScheduledOutcomeChangeData {
    preview_scheduled_outcome_change?: IChangeApplies | null
}

/** Saves the lifecycle policies, so the change is logged and the predictions recomputed. */
export const SAVE_LIFECYCLE_POLICIES = gql`
    mutation SaveLifecyclePolicies($electionEventId: uuid!, $policies: jsonb!) {
        save_lifecycle_policies(election_event_id: $electionEventId, policies: $policies) {
            policies
            applies
            applies_message_key
            changes
        }
    }
`
export interface SaveLifecyclePoliciesData {
    save_lifecycle_policies?: IChangeApplies | null
}

export const PREVIEW_SCHEDULE_IMPORT = gql`
    mutation PreviewScheduleImport($electionEventId: uuid!, $documentId: String!) {
        preview_schedule_import(election_event_id: $electionEventId, document_id: $documentId) {
            rows {
                row
                election_alias
                election_id
                election_name
                event_type
                local
                time_zone
                instant
                primary_local
                error_code
                note_code
            }
            ok
            posts
            errors
            primary_time_zone
            outcome_changes
        }
    }
`
export interface PreviewScheduleImportData {
    preview_schedule_import?: IScheduleImportPreview | null
}

export const IMPORT_SCHEDULE = gql`
    mutation ImportSchedule($electionEventId: uuid!, $documentId: String!) {
        import_schedule(election_event_id: $electionEventId, document_id: $documentId) {
            created
            updated
        }
    }
`
export interface ImportScheduleData {
    import_schedule?: {created: number; updated: number} | null
}

export const EXPORT_SCHEDULE = gql`
    mutation ExportSchedule($electionEventId: uuid!) {
        export_schedule(election_event_id: $electionEventId) {
            document_id
        }
    }
`
export interface ExportScheduleData {
    export_schedule?: {document_id: string} | null
}

export const APPLY_SCHEDULE_RECOMPUTE = gql`
    mutation ApplyScheduleRecompute($electionEventId: uuid!) {
        apply_schedule_recompute(election_event_id: $electionEventId) {
            updated
        }
    }
`
export interface ApplyScheduleRecomputeData {
    apply_schedule_recompute?: {updated: number} | null
}

/**
 * The published lifecycle configuration of each target (the event, and each
 * election published on its own), as windmill keeps it where admins can't
 * change it. Per target: the newest kept snapshot first, then the newest
 * signed one when that is a different publication.
 */
export const GET_LIFECYCLE_SNAPSHOTS = gql`
    query GetLifecycleSnapshots($electionEventId: uuid!) {
        get_lifecycle_snapshots(election_event_id: $electionEventId) {
            snapshots
        }
    }
`
export interface ILifecycleSnapshotEntry {
    /** The published election; null for the event. */
    election_id: string | null
    publication_id: string
    /** RFC 3339: when the snapshot was kept. */
    published_at: string
    approval_request_id: string | null
    approval_code: string | null
    /** Whether an executed configuration approval signed this snapshot. */
    signed: boolean
    snapshot: ILifecycleSnapshot
}
export interface GetLifecycleSnapshotsData {
    get_lifecycle_snapshots?: {snapshots: Array<ILifecycleSnapshotEntry>} | null
}

/** The newest snapshot of each target (the published copy), targets newest first. */
export const newestPerTarget = (
    entries: ReadonlyArray<ILifecycleSnapshotEntry>
): Array<ILifecycleSnapshotEntry> => {
    const seen = new Set<string>()
    return entries.filter((entry) => {
        const target = entry.election_id ?? ""
        if (seen.has(target)) return false
        seen.add(target)
        return true
    })
}

/**
 * A configuration approval, and the executed ones before it: the previous
 * approved configuration is the newest whose target applies to it.
 */
export const GET_CONFIGURATION_APPROVALS = gql`
    query GetConfigurationApprovals($electionEventId: uuid!, $requestId: uuid!) {
        current: sequent_backend_signing_request_by_pk(id: $requestId) {
            id
            scope_key
        }
        approvals: sequent_backend_signing_request(
            where: {
                election_event_id: {_eq: $electionEventId}
                action: {_eq: "approve-configuration"}
                status: {_eq: "executed"}
                id: {_neq: $requestId}
            }
            order_by: [{executed_at: desc}, {id: asc}]
        ) {
            id
            code
            scope_key
            subject
            executed_at
        }
    }
`
export interface IConfigurationApproval {
    id: string
    code: string
    scope_key: string
    subject: Record<string, unknown>
    executed_at?: string | null
}
export interface GetConfigurationApprovalsData {
    current?: {id: string; scope_key: string} | null
    approvals: Array<IConfigurationApproval>
}
