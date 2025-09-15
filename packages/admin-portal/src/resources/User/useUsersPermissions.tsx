// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useTenantStore} from "@/providers/TenantContextProvider"
import {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

export function useUsersPermissions() {
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    /**
     * Permissions
     */
    const canImportUsers = authContext.isAuthorized(true, tenantId, IPermissions.USER_IMPORT)

    const canExportVoters = authContext.isAuthorized(true, tenantId, IPermissions.VOTER_EXPORT)
    const canCreateVoters = authContext.isAuthorized(true, tenantId, IPermissions.VOTER_CREATE)
    const canEditVoters = authContext.isAuthorized(true, tenantId, IPermissions.VOTER_WRITE)
    const canEditVotersWhoVoted = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.VOTER_VOTED_EDIT
    )
    const canEditVotersEmailTlf = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.VOTER_EMAIL_TLF_EDIT
    )
    const canDeleteVoters = authContext.isAuthorized(true, tenantId, IPermissions.VOTER_DELETE)
    const canImportVoters = authContext.isAuthorized(true, tenantId, IPermissions.VOTER_IMPORT)
    const canManuallyVerify = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.VOTER_MANUALLY_VERIFY
    )
    const canChangePassword = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.VOTER_CHANGE_PASSWORD
    )

    const showVotersColumns = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.EE_VOTERS_COLUMNS
    )
    const showVotersFilters = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.EE_VOTERS_FILTERS
    )
    const showVotersLogs = authContext.isAuthorized(true, tenantId, IPermissions.EE_VOTERS_LOGS)
    const canSendTemplates = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.NOTIFICATION_SEND
    )

    const canImportVotersDelegations = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.VOTER_DELEGATION_IMPORT
    )
    /**
     * Permissions
     */

    return {
        canImportUsers,
        canCreateVoters,
        canEditVoters,
        canEditVotersWhoVoted,
        canEditVotersEmailTlf,
        canDeleteVoters,
        canImportVoters,
        canExportVoters,
        canManuallyVerify,
        canChangePassword,
        showVotersColumns,
        showVotersFilters,
        showVotersLogs,
        canSendTemplates,
        canImportVotersDelegations,
    }
}
