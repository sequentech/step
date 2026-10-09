// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {EChecksPeriodPolicy, IElectionEventPresentation} from "../types/ElectionEventPresentation"

export enum EChecksPeriodStatus {
    UNLIMITED = "unlimited",
    OPEN = "open",
    ENDED = "ended",
    INVALID = "invalid",
}

export type ChecksPeriod =
    | {status: EChecksPeriodStatus.UNLIMITED | EChecksPeriodStatus.INVALID}
    | {status: EChecksPeriodStatus.OPEN | EChecksPeriodStatus.ENDED; until: Date}

// The backend reads the date as RFC 3339, which requires seconds and an offset.
const RFC3339_DATE_TIME = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$/i

const parseChecksAvailableUntil = (value?: string | null): Date | null => {
    const text = value?.trim() ?? ""
    if (!RFC3339_DATE_TIME.test(text)) {
        return null
    }
    const date = new Date(text)
    return Number.isNaN(date.getTime()) ? null : date
}

export const isValidChecksAvailableUntil = (value?: string | null): boolean =>
    parseChecksAvailableUntil(value) !== null

/**
 * The period in which voters can view their cast ballots, for display. The
 * backend decides with its own clock; this mirrors it so the portal can say
 * until when, or that checks have ended, without a failed request.
 */
export const getChecksPeriod = (
    presentation: IElectionEventPresentation | null | undefined,
    now: Date
): ChecksPeriod => {
    const receipts = presentation?.receipts
    if (receipts?.checks_period_policy !== EChecksPeriodPolicy.UNTIL_DATE) {
        return {status: EChecksPeriodStatus.UNLIMITED}
    }
    const until = parseChecksAvailableUntil(receipts.checks_available_until)
    if (!until) {
        return {status: EChecksPeriodStatus.INVALID}
    }
    return {
        status:
            now.getTime() > until.getTime() ? EChecksPeriodStatus.ENDED : EChecksPeriodStatus.OPEN,
        until,
    }
}
