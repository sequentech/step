// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {
    CheckboxGroupInput,
    EditBase,
    Identifier,
    List,
    RecordContext,
    SaveButton,
    SimpleForm,
    TextInput,
    useGetList,
    useListContext,
    useNotify,
    useRefresh,
} from "react-admin"
import {useMutation, useQuery} from "@apollo/client"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {GET_AREAS_EXTENDED} from "@/queries/GetAreasExtended"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {INSERT_AREA_CONTESTS} from "../../queries/InsertAreaContest"
import {DELETE_AREA_CONTESTS} from "@/queries/DeleteAreaContest"
import {IUser} from "sequent-core"
import {EditUserForm} from "./EditUserForm"

interface EditUserProps {
    id?: Identifier | undefined
    electionEventId: Identifier | undefined
    close?: () => void
}

export const EditUser: React.FC<EditUserProps> = (props) => {
    const {id, close, electionEventId} = props

    const {data, isLoading} = useListContext()
    let user: IUser | undefined = data?.find((element) => element.id === id)

    console.log("DATA :: ", data)
    console.log("USER :: ", user)

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
                <EditUserForm id={id} electionEventId={electionEventId} close={close} />
            </List>
        )
    } else {
        return null
    }
}
