// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ETrusteeModePolicy} from "@sequentech/ui-core"

export type TrusteeSessionStatus = "ACTIVE" | "NOT_ACTIVE"

export interface TrusteeSession {
    board_name: string
    sender_pk: string
    trustee_name: string
    trustee_mode: ETrusteeModePolicy
    status: TrusteeSessionStatus
}

export interface SessionsListResponse {
    sessions: TrusteeSession[]
}

/**
 * Fetch trustee sessions from B4 for a given board.
 * Returns null when B4 is unreachable or the response is not OK.
 */
export async function fetchSessions(
    b4Url: string,
    boardName: string,
    heartbeatSecs: number,
    accessToken: string
): Promise<SessionsListResponse | null> {
    try {
        const res = await fetch(
            `${b4Url}/sessions?board_name=${encodeURIComponent(boardName)}&heartbeat_secs=${heartbeatSecs}`,
            {headers: {Authorization: `Bearer ${accessToken}`}}
        )
        if (!res.ok) return null
        return (await res.json()) as SessionsListResponse
    } catch (_) {
        // B4 may not be reachable; ignore silently
        return null
    }
}
