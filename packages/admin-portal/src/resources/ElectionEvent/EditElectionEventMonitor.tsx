// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {MonitorList} from "../Monitor/MonitorList"

export interface EditElectionEventMonitorProps {
    electionEventId?: string
}

export const EditElectionEventMonitor: React.FC<EditElectionEventMonitorProps> = ({
    electionEventId,
}) => {
    // const authContext = useContext(AuthContext)
    // const [tenantId] = useTenantStore()
    // const showUsers = authContext.isAuthorized(true, tenantId, IPermissions.MONITOR_READ)

    // if (!showUsers) {
    //     return null
    // }

    return <MonitorList electionEventId={electionEventId} />
}
