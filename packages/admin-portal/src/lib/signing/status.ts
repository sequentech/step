// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// How a request's status reads, wherever it shows: the panel's chip and the
// Signatures tab's Requests list name each status the same way.

import {SigningRequestStatus} from "./types"

/** The status a request shows: a waiting one past its expiry is expired, before the job says so. */
export function shownRequestStatus(
    request: {status: SigningRequestStatus; expires_at?: string | null},
    now: Date
): SigningRequestStatus {
    return request.status === SigningRequestStatus.Waiting &&
        request.expires_at &&
        Date.parse(request.expires_at) <= now.getTime()
        ? SigningRequestStatus.Expired
        : request.status
}

/** The translation key of a status's label. */
export const requestStatusKey = (status: SigningRequestStatus): string => `signing.status.${status}`
