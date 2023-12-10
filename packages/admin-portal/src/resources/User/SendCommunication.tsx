// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {List, useListContext} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IRole} from "sequent-core"
import {SendCommunicationForm} from "./SendCommunicationForm"

interface SendCommunicationProps {
    id?: string
    electionEventId?: string
    close?: () => void
}

export const SendCommunication: React.FC<SendCommunicationProps> = ({
    id, close, electionEventId
}) => {
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
                <SendCommunicationForm
                    id={id}
                    electionEventId={electionEventId}
                    close={close}
                />
            </List>
        )
    } else {
        return null
    }
}
