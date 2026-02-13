// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useTenantStore} from "@/providers/TenantContextProvider"
import {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

export function usePublishPermissions() {
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    /**
     * Permissions
     */
    const canReadPublish = authContext.isAuthorized(true, tenantId, IPermissions.PUBLISH_READ)
    const canWritePublish = authContext.isAuthorized(true, tenantId, IPermissions.PUBLISH_WRITE)

    const canPublishCreate = authContext.isAuthorized(true, tenantId, IPermissions.PUBLISH_CREATE)
    const canPublishRegenerate = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.PUBLISH_REGENERATE
    )
    const canPublishExport = authContext.isAuthorized(true, tenantId, IPermissions.PUBLISH_EXPORT)
    const canPublishStartVoting = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.PUBLISH_START_VOTING
    )
    const canPublishPauseVoting = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.PUBLISH_PAUSE_VOTING
    )
    const canPublishStopVoting = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.PUBLISH_STOP_VOTING
    )
    const canPublishChanges = authContext.isAuthorized(true, tenantId, IPermissions.PUBLISH_CHANGES)

    const showPublishColumns = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.EE_PUBLISH_COLUMNS
    )
    const showPublishFilters = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.EE_PUBLISH_FILTERS
    )
    const showPublishView = authContext.isAuthorized(true, tenantId, IPermissions.EE_PUBLISH_VIEW)
    const showPublishPreview = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.EE_PUBLISH_PREVIEW
    )
    const showPublishButtonBack = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.EE_PUBLISH_BACK_BUTTON
    )
    /**
     * Permissions
     */

    return {
        canReadPublish,
        canWritePublish,
        canPublishCreate,
        canPublishRegenerate,
        canPublishExport,
        canPublishStartVoting,
        canPublishPauseVoting,
        canPublishStopVoting,
        canPublishChanges,
        showPublishPreview,
        showPublishView,
        showPublishButtonBack,
        showPublishColumns,
        showPublishFilters,
    }
}
