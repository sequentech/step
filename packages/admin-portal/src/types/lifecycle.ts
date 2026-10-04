// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the VOTE-LIFECYCLE Harvest actions answer (design §5a–§6). The Hasura
// actions carry these as jsonb; the generated GraphQL types follow at integration.
import type {
    EInitializeReportPolicy,
    ILifecyclePolicies,
    IScheduledOutcomeExplanation,
} from "@sequentech/ui-core"

/** A signing rule as a publication recorded it. */
export interface IRuleSnapshot {
    required: boolean
    signatures?: number | null
    digest?: string
}

/** A scheduled opening or closing as a publication (and its approval) recorded it. */
export interface IScheduledTransition {
    scheduled_event_id: string
    event_processor: string
    election_id?: string | null
    scheduled_date?: string | null
    local?: string | null
    timezone?: string | null
    voting_channels?: Array<string> | null
    fingerprint: string
}

/** What a publication recorded of the lifecycle (sequent-core `LifecycleSnapshot`). */
export interface ILifecycleSnapshot {
    policies?: ILifecyclePolicies | null
    open_voting?: IRuleSnapshot | null
    close_voting?: IRuleSnapshot | null
    schedule?: Array<IScheduledTransition> | null
    initialization_report_policies?: Record<string, EInitializeReportPolicy>
}

/**
 * `get_scheduled_outcomes`: the predicted outcome of each future scheduled
 * opening and closing, one per row and Post (an event-wide row has one per Post).
 */
export interface IScheduledOutcomeRow {
    scheduled_event_id: string
    election_id?: string | null
    explanation: IScheduledOutcomeExplanation
}

/** `preview_scheduled_outcome_change`: a transition whose outcome the pending change alters. */
export interface IScheduledOutcomeChange {
    /** `new` for the row a create would add. */
    scheduled_event_id: string
    election_id?: string | null
    /** Null for the row a create would add. */
    before: IScheduledOutcomeExplanation | null
    after: IScheduledOutcomeExplanation
}

/** The pending change `preview_scheduled_outcome_change` evaluates, as it would be saved. */
export interface IPendingLifecycleChange {
    /** A scheduled event as it would be saved; a new one has no id. */
    scheduled_event?: {
        id?: string | null
        event_processor: string
        cron_config: {scheduled_date: string; local: string; timezone: string}
        event_payload: {election_id: string | null; voting_channels?: Array<string> | null}
    }
    policies?: ILifecyclePolicies
    rule?: {action: string; required: boolean; signatures?: number | null}
}

/** A row of `preview_schedule_import`. */
export interface IScheduleImportRow {
    row: number
    election_alias: string
    event_type: string
    local: string
    time_zone: string
    instant?: string | null
    primary_local?: string | null
    error_code?: string | null
    election_id?: string | null
    election_name?: string | null
    /** Not an error: `dst-overlap` (the first occurrence is used). */
    note_code?: string | null
}

export interface IScheduleImportPreview {
    rows: Array<IScheduleImportRow>
    ok: number
    posts: number
    errors: number
    primary_time_zone?: string
    outcome_changes?: Array<IScheduledOutcomeChange>
}

/**
 * A future time a tz database update moved: `scheduled_event.annotations.schedule_recompute`.
 * Nothing changes until it is applied.
 */
export interface IScheduleRecompute {
    /** The instant the current tz database gives for the wall time and zone. */
    scheduled_date: string
    /** The stored instant. */
    previous: string
    local?: string
    timezone?: string
    checked_at?: string
}

/** A warning of `manage_election_dates` (30-day rule, FTL lead time); it never blocks. */
export interface IScheduleWarning {
    code: string
    election_id?: string | null
    message_key: string
    params?: Record<string, unknown> | null
}

/** A close request a scheduled close cancelled (`CancelReason::ClosedOnSchedule`). */
export interface IFiredCancelledRequest {
    /** The cancelled request's action (a close, or the opposite of the change). */
    action?: string
    request_id: string
    code: string
    signatures: number
    required: number
}

/**
 * What a scheduled opening or closing did at one Post when it fired, as windmill
 * records it (`scheduled_event.annotations.fired_outcome.posts[]`).
 */
export interface IFiredPost {
    /** This Post may have opened after other Posts finished initialization. */
    fired_at?: string
    action: string
    election_id?: string | null
    scheduled_event_id: string
    outcome: IScheduledOutcomeExplanation["outcome"]
    authorized_by?: IScheduledOutcomeExplanation["authorized_by"]
    unsigned?: boolean
    fingerprint?: string
    /** The channels the transition changes at the Post. */
    channels?: Array<string>
    /** No channel could change: there was nothing to open or close. */
    nothing_to_change?: boolean
    /** For a refusal: `signing-required` or `not-in-signed-configuration`. */
    reason?: string
    explanation?: IScheduledOutcomeExplanation
    /** A close's seal record. */
    record?: Record<string, unknown>
    cancelled?: Array<IFiredCancelledRequest>
}

/** `scheduled_event.annotations.fired_outcome`. */
export interface IFiredOutcome {
    /** RFC 3339: when it fired. */
    at: string
    posts: Array<IFiredPost>
}

/** A close retained by the executed configuration approval, independently of its live row. */
export interface IRetainedSignedClose {
    scheduled_event_id: string
    election_id: string
    fingerprint: string
    /** The immutable signed UTC instant; current zone labels only convert its display. */
    scheduled_at: string
    /** Channels still covered now; these do not describe historical changes. */
    channels: Array<string>
    authorized_by: NonNullable<IScheduledOutcomeExplanation["authorized_by"]>
    /** Processing was recorded; this alone does not prove a voting state changed. */
    fired_at?: string | null
}
