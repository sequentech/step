// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useTenantStore} from "@/providers/TenantContextProvider"
import {useContext} from "react"
import {useGetOne} from "react-admin"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {signedPackagesRequired} from "@/services/configurationSigningPolicy"

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

    // Where signed packages are required, the other dates arrive only through the signed import.
    const {data: tenant} = useGetOne(
        "sequent_backend_tenant",
        {id: tenantId},
        {enabled: !!tenantId}
    )
    // A tenant without the setting requires them too, and so does one still loading.
    const onlyLockdownPeriod = signedPackagesRequired(tenant)

    return {
        canWriteScheduledEvent,
        canCreateScheduledEvent,
        canDeleteScheduledEvent,
        showScheduledEventColumns,
        onlyLockdownPeriod,
    }
}
