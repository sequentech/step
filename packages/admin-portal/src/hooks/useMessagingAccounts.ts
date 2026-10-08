// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useContext} from "react"
import {useQuery} from "@apollo/client"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {GET_MESSAGING_ACCOUNTS} from "@/queries/GetMessagingAccounts"
import {IPermissions} from "@/types/keycloak"
import {IMessagingAccount} from "@/types/messaging"

export interface IGetMessagingAccountsResult {
    sequent_backend_messaging_account: IMessagingAccount[]
}

/** The tenant's sending accounts, read with the messaging-account-read role when held. */
export const useMessagingAccounts = (options: {skip?: boolean} = {}) => {
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const canRead = authContext.isAuthorized(true, tenantId, IPermissions.MESSAGING_ACCOUNT_READ)
    const {data, loading, error, refetch} = useQuery<IGetMessagingAccountsResult>(
        GET_MESSAGING_ACCOUNTS,
        {
            variables: {tenantId},
            skip: options.skip || !tenantId,
            context: canRead
                ? {headers: {"x-hasura-role": IPermissions.MESSAGING_ACCOUNT_READ}}
                : undefined,
        }
    )
    return {
        accounts: data?.sequent_backend_messaging_account ?? [],
        loading,
        error,
        refetch,
        canRead,
    }
}
