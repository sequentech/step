// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import {ListUsers} from "@/resources/User/ListUsers"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {useRecordContext} from "react-admin"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"

export interface EditElectionEventMonitorProps {
    electionEventId?: string
}

export const EditElectionEventMonitor: React.FC<EditElectionEventMonitorProps> = (
    {electionEventId}
) => {
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const showUsers = authContext.isAuthorized(true, tenantId, IPermissions.VOTER_READ)

    if (!showUsers) {
        return null
    }

    return <ListUsers electionEventId={electionEventId}/>
}
