// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * What a scheduled opening or closing will do, and why (VOTE-LIFECYCLE design
 * §5a–§5c). Mirrors sequent-core `types::scheduled_outcome`.
 */
export enum EScheduledOutcomeKind {
    RUNS = "runs",
    RUNS_UNSIGNED = "runs-unsigned",
    REFUSED = "refused",
    WAITING_FOR_INITIALIZATION = "waiting-for-initialization",
}

export enum EScheduledOutcomeCheckId {
    NEEDS_SIGNATURES = "needs-signatures",
    COVERED = "covered",
    UNSIGNED_CLOSE = "unsigned-close",
    STRICTER_COPY = "stricter-copy",
    DEFAULTS = "defaults",
    INITIALIZATION = "initialization",
    VOTING_CLOSE = "voting-close",
}

/** An i18n message key with its parameters. */
export interface IScheduledOutcomeValue {
    message_key: string
    params?: Record<string, unknown>
}

export interface IScheduledOutcomeCheck {
    id: EScheduledOutcomeCheckId
    current: IScheduledOutcomeValue
    published?: IScheduledOutcomeValue | null
    allows: boolean
}

export interface IScheduledOutcomeAuthorizedBy {
    request_id: string
    code: string
    signers: Array<string>
}

export interface IScheduledOutcomeExplanation {
    outcome: EScheduledOutcomeKind
    checks: Array<IScheduledOutcomeCheck>
    deciding: EScheduledOutcomeCheckId
    next_step: IScheduledOutcomeValue
    authorized_by?: IScheduledOutcomeAuthorizedBy | null
}
