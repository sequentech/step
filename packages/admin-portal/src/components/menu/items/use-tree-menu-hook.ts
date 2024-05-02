// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useTenantStore} from "@/providers/TenantContextProvider"
import {FETCH_ELECTION_EVENTS_TREE} from "@/queries/GetElectionEventsTree"
import {useQuery} from "@apollo/client"
import {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {SettingsContext} from "@/providers/SettingsContextProvider"

export function useTreeMenuData(isArchivedElectionEvents: boolean) {
    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)

    return useQuery(FETCH_ELECTION_EVENTS_TREE, {
        variables: {
            tenantId: tenantId,
            isArchived: isArchivedElectionEvents,
        },
        pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
    })
}

export function useActionPermissions() {
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    const canCreateElectionEvent = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.ELECTION_EVENT_CREATE
    )
    const canEditElectionEvent = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.ELECTION_EVENT_WRITE
    )

    return {
        canCreateElectionEvent,
        canEditElectionEvent,
    }
}
