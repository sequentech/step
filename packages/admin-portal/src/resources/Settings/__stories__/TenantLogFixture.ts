// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {STORY_IDS} from "@/__stories__/fixtures"

/** A row of the tenant's log, as the list action answers it. */
export interface TenantLogRecord {
    id: number
    created: number
    statement_timestamp: number
    statement_kind: string
    message: string
    user_id: string | null
    username: string | null
}

const FIRST_ENTRY_AT = 1767225600

export const ADMIN = {id: STORY_IDS.user, name: "ada.admin"}
export const DELETED_EVENT_ID = STORY_IDS.event
export const LOG_BOARD = "devtenant90505c8a23a94cdfa26b4e19f6a097d5"
export const LOG_PUBLIC_KEY = "MCowBQYDK2VwAyEAu0c1q6c9l0c3Jm2dX1o8lq0yq7d0sY1mJrHkq3hXq2s="

const row = (
    id: number,
    kind: string,
    user: {id: string; name: string} | null,
    head: {event_type: string; log_type: string; description: string},
    body: Record<string, unknown>
): TenantLogRecord => ({
    id,
    created: FIRST_ENTRY_AT + id * 60 + 1,
    statement_timestamp: FIRST_ENTRY_AT + id * 60,
    statement_kind: kind,
    user_id: user?.id ?? null,
    username: user?.name ?? null,
    message: JSON.stringify({
        user_id: user?.id ?? null,
        username: user?.name ?? null,
        statement: {head: {event: "Generic Event", kind, ...head}, body},
    }),
})

const activity = (
    kind: string,
    head: Record<string, string>,
    details: Record<string, unknown>
) => ({
    TenantActivity: {
        tenant_id: STORY_IDS.tenant,
        kind,
        ...head,
        details_json: JSON.stringify(details),
    },
})

const DENIED = {
    event_type: "USER",
    log_type: "ERROR",
    description: "Request to delete an election event denied.",
}
const SIGNED_IN = {event_type: "USER", log_type: "INFO", description: "LOGIN"}
const SIGN_IN_FAILED = {
    event_type: "USER",
    log_type: "ERROR",
    description: "LOGIN_ERROR invalid_user_credentials",
}
const OPENED = {event_type: "SYSTEM", log_type: "INFO", description: "Opened the tenant log."}

/** A tenant's log, newest first: opened, a failed sign-in, a sign-in and a refused deletion. */
export const tenantLogRecords = (): TenantLogRecord[] => [
    row(
        4,
        "TenantAdminAction",
        ADMIN,
        DENIED,
        activity("TenantAdminAction", DENIED, {
            action: "delete-election-event",
            outcome: "denied",
            status: 401,
            user_id: ADMIN.id,
            username: ADMIN.name,
            details: {election_event_id: DELETED_EVENT_ID},
        })
    ),
    row(3, "KeycloakUserEvent", ADMIN, SIGNED_IN, {KeycloakUserEvent: ["null", "LOGIN"]}),
    row(2, "KeycloakUserEvent", {id: "unknown_user", name: "null"}, SIGN_IN_FAILED, {
        KeycloakUserEvent: ["invalid_user_credentials", "LOGIN_ERROR"],
    }),
    row(
        1,
        "TenantLogOpened",
        null,
        OPENED,
        activity("TenantLogOpened", OPENED, {board: LOG_BOARD, public_key: LOG_PUBLIC_KEY})
    ),
]
