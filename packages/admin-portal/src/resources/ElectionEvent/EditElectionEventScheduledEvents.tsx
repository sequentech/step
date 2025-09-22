// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import ListScheduledEvents from "../ScheduledEvents/ListScheduledEvent"

interface EditElectionEventsProps {
    electionEventId: string
}
export const EditElectionEventEvents: React.FC<EditElectionEventsProps> = ({electionEventId}) => {
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const showEvents = authContext.isAuthorized(true, tenantId, IPermissions.ELECTION_EVENT_READ)

    if (!showEvents) {
        return null
    }

    return <ListScheduledEvents electionEventId={electionEventId} />
}
