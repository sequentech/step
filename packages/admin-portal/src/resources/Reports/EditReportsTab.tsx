// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import ListReports from "./ListReports"

interface EditReportsProps {
    electionEventId: string
}
export const EditReportsTab: React.FC<EditReportsProps> = ({electionEventId}) => {
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const showReports = authContext.isAuthorized(true, tenantId, IPermissions.REPORT_READ)

    if (!showReports) {
        return null
    }

    return <ListReports electionEventId={electionEventId} />
}
