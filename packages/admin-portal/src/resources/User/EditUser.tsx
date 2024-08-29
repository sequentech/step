// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, { useEffect, useState } from "react"
import { List, useListContext } from "react-admin"
import { useTenantStore } from "@/providers/TenantContextProvider"
import { IRole, IUser } from "@sequentech/ui-core"
import { EditUserForm } from "./EditUserForm"
import { UserProfileAttribute } from "@/gql/graphql"

interface EditUserProps {
    id?: string
    electionEventId?: string
    close?: () => void
    rolesList: Array<IRole>
    userAttributes: UserProfileAttribute[]
}

export const EditUser: React.FC<EditUserProps> = ({ id, close, electionEventId, rolesList, userAttributes }) => {
    const { data, isLoading } = useListContext()

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
                filter={{ tenant_id: tenantId, election_event_id: electionEventId }}
                sx={{ padding: "16px" }}
                actions={false}
            >
                <EditUserForm
                    id={id}
                    electionEventId={electionEventId}
                    close={close}
                    rolesList={rolesList}
                    userAttributes={userAttributes}
                />
            </List>
        )
    } else {
        return null
    }
}
