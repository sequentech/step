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
import { EditUserForm } from './EditUserForm'

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

    const refresh = useRefresh()
    const notify = useNotify()
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()

    const [renderUI, setRenderUI] = useState(true)

    const {data: contests} = useGetList("sequent_backend_contest", {
        pagination: {page: 1, perPage: 9999},
        filter: {election_event_id: electionEventId},
    })

    useEffect(() => {
        if (isLoading && data) {
            setRenderUI(true)
        }
    }, [isLoading, data])

    const parseValues = (incoming: any) => {
        const temp = {...incoming}

        // temp.area_contest_ids = areas?.sequent_backend_area_contest?.map(
        //     (area: any) => area.contest.id
        // )

        return temp
    }

    const transform = async (data: any, {previousData}: any) => {
        const temp = {...data}
        return temp

        // const area_contest_ids = temp.area_contest_ids
        // const election_event_id = temp.election_event_id
        // const area_id = temp.id

        // // delete area contest first
        // let {errors: deleteAreasErrors} = await delete_sequent_backend_area_contest({
        //     variables: {
        //         tenantId,
        //         area: temp.id,
        //     },
        // })

        // if (deleteAreasErrors) {
        //     console.log("deleteAreasErrors :>> ", deleteAreasErrors)
        //     notify("Could not update Area", {type: "error"})
        //     return
        // }

        // const area_contest_ids_to_save = area_contest_ids?.map((contest_id: any) => {
        //     return {
        //         area_id,
        //         contest_id,
        //         election_event_id,
        //         tenant_id: tenantId,
        //     }
        // })

        // // delete area contest first
        // let {errors: insertAreasErrors} = await insert_sequent_backend_area_contest({
        //     variables: {
        //         areas: area_contest_ids_to_save,
        //     },
        // })

        // if (insertAreasErrors) {
        //     console.log("insertAreasErrors :>> ", insertAreasErrors)
        //     notify("Could not update Area", {type: "error"})
        //     return
        // }

        // delete temp.area_contest_ids
        // console.log("DATA TO SAVE :: ", area_contest_ids_to_save)

        // if (shallowEqual(temp, previousData)) {
        //     console.log("NO CHANGES")
        //     return {id: temp.id, last_updated_at: new Date().toISOString()}
        // }
        // return {...temp, last_updated_at: new Date().toISOString()}
    }

    const onSuccess = async (res: any) => {
        console.log("onSuccess :>> ", res)

        refresh()
        notify("User updated", {type: "success"})
        if (close) {
            setTimeout(() => {
                close()
            }, 400)
        }
    }

    const onError = async (res: any) => {
        console.log("onError :>> ", res)

        refresh()
        notify("Could not update User", {type: "error"})
        if (close) {
            setTimeout(() => {
                close()
            }, 400)
        }
    }
    

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
