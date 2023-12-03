// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useGetList} from "react-admin"
import React, {ReactElement, useEffect} from "react"

import {useNavigate} from "react-router-dom"
import {CircularProgress} from "@mui/material"
import {useTenantStore} from "@/providers/TenantContextProvider"

export interface ElectionEventListProps {
    aside?: ReactElement
}

export const ElectionEventList: React.FC<ElectionEventListProps> = ({aside}) => {
    const navigate = useNavigate()
    const [tenantId] = useTenantStore()

    const {data} = useGetList("sequent_backend_election_event", {
        sort: {field: "created_at", order: "DESC"},
        filter: {
            tenant_id: tenantId,
        },
    })

    // Navigate to the first election event found, if any
    useEffect(() => {
        if (data && data.length > 0) {
            const electionEventId = data[0].id ?? null
            if (electionEventId) {
                navigate("/sequent_backend_election_event/" + electionEventId)
            }
        }
    })

    // if data, we would be automatically redirected to the first election
    // event, so we should just show a process icon in the meantime
    return <CircularProgress />
}
