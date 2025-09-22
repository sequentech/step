// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useTenantStore} from "@/providers/TenantContextProvider"
import {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

export function useScheduledEventPermissions() {
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    /**
     * Permissions
     */
    const canWriteScheduledEvent = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.SCHEDULED_EVENT_WRITE
    )
    const canCreateScheduledEvent = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.SCHEDULED_EVENT_CREATE
    )
    const canDeleteScheduledEvent = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.SCHEDULED_EVENT_DELETE
    )

    const showScheduledEventColumns = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.EE_SCHEDULED_EVENT_COLUMNS
    )
    /**
     * Permissions
     */

    return {
        canWriteScheduledEvent,
        canCreateScheduledEvent,
        canDeleteScheduledEvent,
        showScheduledEventColumns,
    }
}
