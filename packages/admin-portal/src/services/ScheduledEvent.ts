// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum ScheduledEventType {
    CREATE_REPORT = "CREATE_REPORT",
    UPDATE_VOTING_STATUS = "UPDATE_VOTING_STATUS",
    CREATE_BOARD = "CREATE_BOARD",
    CREATE_KEYS = "CREATE_KEYS",
    SET_PUBLIC_KEY = "SET_PUBLIC_KEY",
    CREATE_ELECTION_EVENT_BALLOT_STYLES = "CREATE_ELECTION_EVENT_BALLOT_STYLES",
    TALLY_ELECTION_EVENT = "TALLY_ELECTION_EVENT",
    SEND_TEMPLATE = "SEND_TEMPLATE",
}

/**
 * The list filter of scheduled events that are still events: not archived.
 * A bulk notification (Send Notification) is stored as a stopped, archived
 * `SEND_TEMPLATE` row, kept only as the content its retries are sent from,
 * so it is never listed, edited or rescheduled here.
 */
export const ACTIVE_SCHEDULED_EVENTS = {
    archived_at: {format: "hasura-raw-query", value: {_is_null: true}},
} as const
