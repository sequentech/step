// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Identifier, List} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {EditUserForm} from "./EditUserForm"

interface EditUserProps {
    id?: Identifier | undefined
    electionEventId: Identifier | undefined
    close?: () => void
}

export const EditUser: React.FC<EditUserProps> = (props) => {
    const {id, close, electionEventId} = props
    const [tenantId] = useTenantStore()

    return (
        <List
            resource="user"
            filter={{tenant_id: tenantId, election_event_id: electionEventId}}
            actions={<></>}
            sx={{padding: "0 16px"}}
        >
            <EditUserForm id={id} electionEventId={electionEventId} close={close} />
        </List>
    )
}
