// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"

/**
 * The role messaging data is read with outside Settings > Messaging: the event
 * configuration, safe account columns and the message ledger. `null` when the
 * user has none, so the screens skip the query and say why instead of falling
 * back to the default role, which can't read them.
 */
export const useMessagingReadRole = (): IPermissions | null => {
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    if (authContext.isAuthorized(true, tenantId, IPermissions.MESSAGING_CONFIG_READ)) {
        return IPermissions.MESSAGING_CONFIG_READ
    }
    return null
}

export const roleContext = (role: IPermissions | null) =>
    role ? {headers: {"x-hasura-role": role}} : undefined
