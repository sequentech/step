// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {get_auth_url_js} from "sequent-core"

export const getAuthUrl = (
    baseUrl: string,
    tenantId: string,
    eventId: string,
    authAction: "login" | "enroll"
): string | null => {
    try {
        return get_auth_url_js(baseUrl, tenantId, eventId, authAction)
    } catch (error) {
        console.log(error)
        throw error
    }
}
