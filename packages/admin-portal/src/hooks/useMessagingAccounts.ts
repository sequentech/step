// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useContext} from "react"
import {useQuery} from "@apollo/client"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {
    GET_MESSAGING_ACCOUNTS,
    GET_MESSAGING_ACCOUNT_SUMMARIES,
} from "@/queries/GetMessagingAccounts"
import {IPermissions} from "@/types/keycloak"
import {IMessagingAccount} from "@/types/messaging"
import {roleContext, useMessagingReadRole} from "./useMessagingReadRole"

export interface IGetMessagingAccountsResult {
    sequent_backend_messaging_account: IMessagingAccount[]
}

/**
 * The tenant's sending accounts. With messaging-account-read (Settings) they
 * include when each credential was replaced; otherwise only the safe columns
 * are read, with messaging-config-read. Without either, nothing is read and
 * `canRead` is false: the default role can't read accounts.
 */
export const useMessagingAccounts = (options: {skip?: boolean} = {}) => {
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const canReadAccounts = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.MESSAGING_ACCOUNT_READ
    )
    const readRole = useMessagingReadRole()
    const role = canReadAccounts ? IPermissions.MESSAGING_ACCOUNT_READ : readRole
    const {data, loading, error, refetch} = useQuery<IGetMessagingAccountsResult>(
        canReadAccounts ? GET_MESSAGING_ACCOUNTS : GET_MESSAGING_ACCOUNT_SUMMARIES,
        {
            variables: {tenantId},
            skip: options.skip || !tenantId || !role,
            context: roleContext(role),
        }
    )
    return {
        accounts: data?.sequent_backend_messaging_account ?? [],
        loading,
        error,
        refetch,
        canRead: !!role,
    }
}
