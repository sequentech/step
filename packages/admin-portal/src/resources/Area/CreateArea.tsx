// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import {
    SimpleForm,
    TextInput,
    Create,
    useRefresh,
    useNotify,
    Identifier,
    InputPropTypes,
    SaveButton,
    Toolbar,
} from "react-admin"
import {Sequent_Backend_Election_Event, UpsertAreaMutation} from "../../gql/graphql"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {useMutation} from "@apollo/client"
import {INSERT_AREA_CONTESTS} from "@/queries/InsertAreaContest"
import {UPSERT_AREA} from "@/queries/UpsertArea"
import {AreaForm} from "./AreaForm"

interface CreateAreaProps {
    record: Sequent_Backend_Election_Event
    electionEventId: Identifier | undefined
    close?: () => void
}

export const CreateArea: React.FC<CreateAreaProps> = (props) => {
    const {record, electionEventId, close} = props
    const refresh = useRefresh()
    const notify = useNotify()
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()

    const [upsertArea] = useMutation<UpsertAreaMutation>(UPSERT_AREA)

    // const [insert_sequent_backend_area_contest] = useMutation(INSERT_AREA_CONTESTS)
    // const [areaContests, setAreaContests] = useState<Identifier[]>([])
    // const [areaId, setAreaId] = useState<Identifier | null>(null)

    // const onSuccess = (data: any) => {
    //     setAreaId(data.id)
    //     refresh()
    //     notify(t("areas.createAreaSuccess"), {type: "success"})
    //     if (close) {
    //         close()
    //     }
    // }

    // const onError = async (res: any) => {
    //     refresh()
    //     notify("areas.createAreaError", {type: "error"})
    //     if (close) {
    //         close()
    //     }
    // }

    // /**
    //  * Transforms the input data by extracting `area_contest_ids` and updating the state,
    //  * while removing the `area_contest_ids` property from the returned object.
    //  *
    //  * @param data - The input data object to be transformed.
    //  * @returns A new object with the `area_contest_ids` property removed.
    //  */
    // const transform = async (data: any) => {
    //     const temp = {...data}

    //     setAreaContests([...data.area_contest_ids])
    //     delete temp.area_contest_ids
    //     return temp
    // }

    // useEffect(() => {
    //     if (areaId && areaContests.length > 0) {
    //         // when has created the area and has contest in the array, save them
    //         saveContetst()
    //     }
    // }, [areaContests, areaId])

    // /**
    //  * Asynchronously saves the area contests by mapping the provided area contest IDs
    //  * to a structure containing area, contest, election event, and tenant information.
    //  *
    //  * This function performs the following steps:
    //  * 1. Maps the `areaContests` to a list of objects containing the necessary data for saving.
    //  * 2. Sends the mapped data to the backend using the `insert_sequent_backend_area_contest` mutation.
    //  * 3. If an error occurs during the insertion, it notifies the user with an error message.
    //  *
    //  * @async
    //  * @function saveContetst
    //  * @returns {Promise<void>} Resolves when the operation is complete.
    //  *
    //  * @throws {Error} If the backend insertion fails, an error notification is displayed.
    //  */
    // const saveContetst = async () => {
    //     const area_contest_ids = [...areaContests]
    //     const election_event_id = electionEventId
    //     const area_id = areaId

    //     const area_contest_ids_to_save = area_contest_ids?.map((contest_id: any) => {
    //         return {
    //             area_id,
    //             contest_id,
    //             election_event_id,
    //             tenant_id: tenantId,
    //         }
    //     })

    //     // insert area contest first
    //     let {errors: insertAreasErrors} = await insert_sequent_backend_area_contest({
    //         variables: {
    //             areas: area_contest_ids_to_save,
    //         },
    //     })

    //     if (insertAreasErrors) {
    //         notify("Could not update Area")
    //         return
    //     }
    // }

    const onSubmit = async (values: any) => {
        console.log("aa values", values)

        try {
            const {data} = await upsertArea({
                variables: {
                    id: values.id,
                    name: values.name,
                    description: values.description,
                    presentation: values.presentation,
                    tenantId: tenantId,
                    electionEventId,
                    parentId: values.parentId,
                    areaContestsIds: values.area_contest_ids,
                    annotations: values.annotations,
                    labels: values.labels,
                    type: values.type,
                },
            })
            refresh()
            notify(t("areas.createAreaSuccess"), {type: "success"})
            if (close) {
                close()
            }
        } catch (e) {
            console.log("aa error creating", e)
            refresh()
            notify("areas.createAreaError", {type: "error"})
            if (close) {
                close()
            }
        }
    }

    return (
        <Create
            resource="sequent_backend_area"
            // mutationOptions={{onSuccess, onError}}
            redirect={false}
            // transform={transform}
        >
        <PageHeaderStyles.Wrapper>
            <SimpleForm
                onSubmit={onSubmit}
                toolbar={
                    <Toolbar>
                        <SaveButton className="election-save-button" />
                    </Toolbar>
                }
            >
                <AreaForm electionEventId={electionEventId} />

                <TextInput
                    label="Election Event"
                    source="election_event_id"
                    defaultValue={record?.id || ""}
                    style={{display: "none"}}
                />
                <TextInput
                    label="Tenant"
                    source="tenant_id"
                    defaultValue={record?.tenant_id || ""}
                    style={{display: "none"}}
                />
            </SimpleForm>
        </PageHeaderStyles.Wrapper>
        </Create>
    )
}
