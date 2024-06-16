// SPDX-FileCopyrightText: 2023-2024 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {
    CheckboxGroupInput,
    EditBase,
    Identifier,
    RecordContext,
    SaveButton,
    SelectField,
    AutocompleteInput,
    ReferenceInput,
    SimpleForm,
    TextInput,
    useGetList,
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
import {Sequent_Backend_Area} from "@/gql/graphql"
import {keyBy} from "@sequentech/ui-essentials"

interface EditAreaProps {
    id?: Identifier | undefined
    electionEventId: Identifier | undefined
    close?: () => void
}

export const EditArea: React.FC<EditAreaProps> = (props) => {
    const {id, close, electionEventId} = props
    const [areasList, setAreasList] = useState<Array<Sequent_Backend_Area>>([])
    const areaFilterToQuery = (searchText: string) => {
        if (!searchText || searchText.length == 0) {
            return {}
        }
        return {
            // FIXME: this seems to be generating a '%[object Object]%' in the
            // actual graphql query
            name: {_ilike: `%${searchText}%`},

            // FIXME: The idea is filter out of the search the current area. it
            // should not be selectable
            id: {_neq: id},
        }
    }

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

    const {data: contests} = useGetList("sequent_backend_contest", {
        pagination: {page: 1, perPage: 9999},
        filter: {election_event_id: electionEventId},
    })

    const {data: allAreas} = useGetList<Sequent_Backend_Area>("sequent_backend_area", {
        filter: {
            tenant_id: tenantId,
            election_event_id: electionEventId,
        },
    })

    const {data: areas} = useQuery(GET_AREAS_EXTENDED, {
        variables: {
            electionEventId,
            areaId: id,
        },
    })

    useEffect(() => {
        let allAreasDefault = allAreas ?? []
        let areasMap = keyBy(allAreasDefault, "id")

        const isCyclical = (
            childId: Identifier | undefined,
            parent: Sequent_Backend_Area,
            map: Record<string, Sequent_Backend_Area>
        ): boolean => {
            let parents: Array<Identifier> = []
            if (childId) {
                parents.push(childId)
            }
            let current: Sequent_Backend_Area | undefined = parent
            while (current) {
                if (parents.includes(current.id)) {
                    return true
                }
                parents.push(current.id)
                current = current.parent_id ? map[current.parent_id] : undefined
            }
            return false
        }

        let nonCyclicalAreas = allAreasDefault.filter((area) => !isCyclical(id, area, areasMap))

        setAreasList(nonCyclicalAreas)
    }, [allAreas, id])

    useEffect(() => {
        if (contests && areas) {
            setRenderUI(true)
        }
    }, [areas, contests])

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
                                    <>
                                        <PageHeaderStyles.Title>
                                            {t("areas.common.title")}
                                        </PageHeaderStyles.Title>
                                        <PageHeaderStyles.SubTitle>
                                            {t("areas.common.subTitle")}
                                        </PageHeaderStyles.SubTitle>

                                        <TextInput source="name" />
                                        <TextInput source="description" />

                                        {contests ? (
                                            <CheckboxGroupInput
                                                label={t("areas.sequent_backend_area_contest")}
                                                source="area_contest_ids"
                                                choices={contests}
                                                optionText="name"
                                                optionValue="id"
                                                row={false}
                                            />
                                        ) : null}

                                        <ReferenceInput
                                            fullWidth={true}
                                            reference="sequent_backend_area"
                                            source="parent_id"
                                            filter={{
                                                tenant_id: tenantId,
                                                election_event_id: electionEventId,
                                            }}
                                            enableGetChoices={({q}) => q && q.length >= 3}
                                        >
                                            <AutocompleteInput
                                                fullWidth={true}
                                                optionText={(area) => area.name}
                                                filterToQuery={areaFilterToQuery}
                                            />
                                        </ReferenceInput>
                                    </>
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
