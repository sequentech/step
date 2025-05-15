// SPDX-FileCopyrightText: 2023-2024 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {
    EditBase,
    Identifier,
    RecordContext,
    SaveButton,
    SimpleForm,
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
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {AreaForm} from "./AreaForm"
interface EditAreaProps {
    id?: Identifier | undefined
    electionEventId: Identifier | undefined
    close?: () => void
}

export const EditArea: React.FC<EditAreaProps> = (props) => {
    const {id, close, electionEventId} = props
    const [delete_sequent_backend_area_contest] = useMutation(DELETE_AREA_CONTESTS)
    const [insert_sequent_backend_area_contest] = useMutation(INSERT_AREA_CONTESTS, {
        refetchQueries: [
            {
                query: GET_AREAS_EXTENDED,
                variables: {
                    electionEventId,
                    areaId: id,
                },
            },
        ],
    })
    const refresh = useRefresh()
    const notify = useNotify()
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()

    const [renderUI, setRenderUI] = useState(false)

    const {data: areas} = useQuery(GET_AREAS_EXTENDED, {
        variables: {
            electionEventId,
            areaId: id,
        },
    })

    const aliasRenderer = useAliasRenderer()

    const contestFilterToQuery = (searchText: string) => {
        if (!searchText || searchText.length == 0) {
            return {name: ""}
        }
        return {"name@_ilike,alias@_ilike": searchText.trim()}
    }

    useEffect(() => {
        if (areas) {
            setRenderUI(true)
        }
    }, [areas])

    const parseValues = (incoming: any) => {
        const temp = {...incoming}

        temp.area_contest_ids = areas?.sequent_backend_area_contest?.map(
            (area: any) => area.contest.id
        )

        return temp
    }

    function shallowEqual(object1: any, object2: any) {
        const keys1 = Object.keys(object1)
        const keys2 = Object.keys(object2)

        if (keys1.length !== keys2.length) {
            return false
        }

        for (let key of keys1) {
            if (object1[key] !== object2[key]) {
                return false
            }
        }

        return true
    }

    const transform = async (data: any, {previousData}: any) => {
        const temp = {...data}

        console.log("aa area", data);
        

        const area_contest_ids = temp.area_contest_ids
        const election_event_id = temp.election_event_id
        const area_id = temp.id

        // delete area contest first
        let {errors: deleteAreasErrors} = await delete_sequent_backend_area_contest({
            variables: {
                tenantId,
                area: temp.id,
            },
        })

        if (deleteAreasErrors) {
            console.log("deleteAreasErrors :>> ", deleteAreasErrors)
            notify("Could not update Area", {type: "error"})
            return
        }

        const area_contest_ids_to_save = area_contest_ids?.map((contest_id: any) => {
            return {
                area_id,
                contest_id,
                election_event_id,
                tenant_id: tenantId,
            }
        })

        // delete area contest first
        let {errors: insertAreasErrors} = await insert_sequent_backend_area_contest({
            variables: {
                areas: area_contest_ids_to_save,
            },
        })

        if (insertAreasErrors) {
            console.log("insertAreasErrors :>> ", insertAreasErrors)
            notify("Could not update Area")
            return
        }

        delete temp.area_contest_ids
        console.log("DATA TO SAVE :: ", area_contest_ids_to_save)

        if (shallowEqual(temp, previousData)) {
            console.log("NO CHANGES")
            return {id: temp.id, last_updated_at: new Date().toISOString()}
        }
        return {...temp, last_updated_at: new Date().toISOString()}
    }

    const onSuccess = async (res: any) => {
        console.log("onSuccess :>> ", res)

        refresh()
        notify("Area updated", {type: "success"})
        if (close) {
            setTimeout(() => {
                close()
            }, 400)
        }
    }

    const onError = async (res: any) => {
        console.log("onError :>> ", res)

        refresh()
        notify("Could not update Area", {type: "error"})
        if (close) {
            setTimeout(() => {
                close()
            }, 400)
        }
    }

    if (renderUI) {
        return (
            <EditBase
                id={id}
                transform={transform}
                resource="sequent_backend_area"
                mutationMode="pessimistic"
                mutationOptions={{onSuccess, onError}}
                redirect={false}
            >
                <PageHeaderStyles.Wrapper>
                    <RecordContext.Consumer>
                        {(incoming) => {
                            const parsedValue = parseValues(incoming)
                            console.log("parsedValue :>> ", parsedValue)
                            return (
                                <SimpleForm record={parsedValue} toolbar={<SaveButton />}>
                                    <AreaForm electionEventId={electionEventId} />
                                </SimpleForm>
                            )
                        }}
                    </RecordContext.Consumer>
                </PageHeaderStyles.Wrapper>
            </EditBase>
        )
    } else {
        return null
    }
}
