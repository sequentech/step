// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useContext, useMemo} from "react"
import {useQuery} from "@apollo/client"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {
    GET_MESSAGING_ACCOUNTS,
    GET_MESSAGING_ACCOUNT_SUMMARIES,
} from "@/queries/GetMessagingAccounts"
import {
    GET_MESSAGING_CALLBACK_STATUS,
    IGetMessagingCallbackStatusResult,
} from "@/queries/GetMessagingCallbackStatus"
import {withCallbackStatus} from "@/services/messaging"
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
 *
 * `summaries` reads only the safe columns even with messaging-account-read,
 * for screens that need no credential records (least privilege).
 *
 * With `callbackStatus`, each account's check carries its callback status
 * (`messaging_callback_status`):
 * without a callback address, Messenger isn't ready for codes and receipts are
 * unavailable. When that status can't be read, the accounts come without it
 * and the backend's own validation still refuses what it blocks.
 */
export const useMessagingAccounts = (
    options: {skip?: boolean; summaries?: boolean; callbackStatus?: boolean} = {}
) => {
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const canReadAccounts = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.MESSAGING_ACCOUNT_READ
    )
    const readRole = useMessagingReadRole()
    const withCredentials = canReadAccounts && !options.summaries
    const role = withCredentials
        ? IPermissions.MESSAGING_ACCOUNT_READ
        : (readRole ?? (canReadAccounts ? IPermissions.MESSAGING_ACCOUNT_READ : null))
    const {data, loading, error, refetch} = useQuery<IGetMessagingAccountsResult>(
        withCredentials ? GET_MESSAGING_ACCOUNTS : GET_MESSAGING_ACCOUNT_SUMMARIES,
        {
            variables: {tenantId},
            skip: options.skip || !tenantId || !role,
            context: roleContext(role),
        }
    )
    const {data: callbacks} = useQuery<IGetMessagingCallbackStatusResult>(
        GET_MESSAGING_CALLBACK_STATUS,
        {
            skip: !options.callbackStatus || options.skip || !tenantId || !role,
            context: roleContext(role),
        }
    )
    const accounts = useMemo(
        () =>
            withCallbackStatus(
                data?.sequent_backend_messaging_account ?? [],
                callbacks?.messaging_callback_status?.accounts
            ),
        [data, callbacks]
    )
    return {
        accounts,
        loading,
        error,
        refetch,
        canRead: !!role,
    }
}
