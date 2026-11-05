// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useCallback, useContext, useMemo} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {SIGNING_ACTIONS, type SigningAction} from "@/lib/signing/types"
import {IPermissions} from "@/types/keycloak"

export interface ISigningPermissions {
    /** Holds `sign-<action>`. Access to the Post is checked by the server. */
    canSign: (action: SigningAction) => boolean
    /** Holds `signing-requests-cancel`: may cancel anyone's waiting request. */
    canCancel: boolean
    canReadRequests: boolean
    canExportRequests: boolean
}

/** The signed-in user's signing permissions in the selected tenant. */
export const useSigningPermissions = (): ISigningPermissions => {
    const {isAuthorized} = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const has = useCallback(
        (permission: IPermissions) => isAuthorized(true, tenantId, permission),
        [isAuthorized, tenantId]
    )
    return useMemo(
        () => ({
            canSign: (action) => {
                const info = SIGNING_ACTIONS[action]
                return info !== undefined && has(info.signPermission)
            },
            canCancel: has(IPermissions.SIGNING_REQUESTS_CANCEL),
            canReadRequests: has(IPermissions.SIGNING_REQUESTS_READ),
            canExportRequests: has(IPermissions.SIGNING_REQUESTS_EXPORT),
        }),
        [has]
    )
}
