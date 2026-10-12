// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EMessageAttemptState, EMessageChannel} from "@/types/messaging"

/** The event's Logs tab, selected by a link's `tabId`. */
export const LOGS_TAB_ID = "logs"

/** Log filters of the messages a SendCommunications entry records. */
export const LOG_FILTER_CHANNEL = "channel"
export const LOG_FILTER_OUTCOME = "outcome"

const SEND_COMMUNICATIONS = "SendCommunications"

/** The Logs channel filter value of messages with no channel (voters with no contact). */
export const LOG_CHANNEL_NONE = "none"

/**
 * The search of the event page that opens Logs with the failed messages of
 * `channel`. The list reads `filter` and `displayedFilters` from the location,
 * so the filters show and can be cleared like any other.
 */
export const logsFilterSearch = (
    /** `null` for the messages of voters with no contact channel. */
    channel: EMessageChannel | null,
    outcome: EMessageAttemptState = EMessageAttemptState.FAILED
): string =>
    `?${new URLSearchParams({
        tabId: LOGS_TAB_ID,
        filter: JSON.stringify({
            statement_kind: SEND_COMMUNICATIONS,
            [LOG_FILTER_CHANNEL]: channel ?? LOG_CHANNEL_NONE,
            [LOG_FILTER_OUTCOME]: outcome,
        }),
        displayedFilters: JSON.stringify({
            statement_kind: true,
            [LOG_FILTER_CHANNEL]: true,
            [LOG_FILTER_OUTCOME]: true,
        }),
    }).toString()}`
