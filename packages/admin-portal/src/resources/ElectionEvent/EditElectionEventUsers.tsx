// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import {ListUsers} from "@/resources/User/ListUsers"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {useRecordContext} from "react-admin"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"

export interface EditElectionEventUsersProps {
    electionEventId?: string
    electionId?: string
}

export const EditElectionEventUsers: React.FC<EditElectionEventUsersProps> = ({
    electionEventId,
    electionId,
}) => {
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const showUsers = authContext.isAuthorized(true, tenantId, IPermissions.VOTER_READ)

    if (!showUsers) {
        return null
    }

    return <ListUsers electionEventId={electionEventId} electionId={electionId} />
}
