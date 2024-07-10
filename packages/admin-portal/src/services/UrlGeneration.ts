// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {get_login_url_js} from "sequent-core"

export const getLoginUrl = (baseUrl: string, tenantId: string, eventId: string): string | null => {
    try {
        return get_login_url_js(baseUrl, tenantId, eventId)
    } catch (error) {
        console.log(error)
        throw error
    }
}
