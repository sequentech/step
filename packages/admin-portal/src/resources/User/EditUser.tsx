// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {List, useListContext} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IRole, IUser} from "sequent-core"
import {EditUserForm} from "./EditUserForm"

interface EditUserProps {
    id?: string
    electionEventId?: string
    close?: () => void
    rolesList: Array<IRole>
}

export const EditUser: React.FC<EditUserProps> = ({id, close, electionEventId, rolesList}) => {
    const {data, isLoading} = useListContext()
    let user: IUser | undefined = data?.find((element) => element.id === id)

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
            >
                <EditUserForm
                    id={id}
                    electionEventId={electionEventId}
                    close={close}
                    rolesList={rolesList}
                />
            </List>
        )
    } else {
        return null
    }
}
