// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {
    Identifier,
    List,
    useListContext,
} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {EditUserForm} from "./EditUserForm"

interface EditUserProps {
    id?: Identifier | undefined
    electionEventId: Identifier | undefined
    close?: () => void
}

export const EditUser: React.FC<EditUserProps> = (props) => {
    const {id, close, electionEventId} = props

    const {data, isLoading} = useListContext()

    const [tenantId] = useTenantStore()

    const [renderUI, setRenderUI] = useState(true)

    useEffect(() => {
        if (isLoading && data) {
            setRenderUI(true)
        }
    }, [isLoading, data])

    if (renderUI) {
        return (
            <List
                resource="user"
                filter={{tenant_id: tenantId, election_event_id: electionEventId}}
                sx={{padding: "16px"}}
                actions={false}
            >
                <EditUserForm id={id} electionEventId={electionEventId} close={close} />
            </List>
        )
    } else {
        return null
    }
}
