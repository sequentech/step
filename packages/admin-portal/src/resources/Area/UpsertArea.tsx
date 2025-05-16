// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import {Create, Identifier, EditBase} from "react-admin"
import {Sequent_Backend_Election_Event} from "../../gql/graphql"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useQuery} from "@apollo/client"
import {GET_AREAS_EXTENDED} from "@/queries/GetAreasExtended"
import {FormContent} from "./FormContent"

export interface UpsertAreaProps {
    record?: Sequent_Backend_Election_Event
    id?: Identifier | undefined
    electionEventId: Identifier | undefined
    close?: () => void
}

/**
 * Component for creating or editing an Area resource.
 *
 * This component handles both the creation and editing of an Area, depending on whether an `id` is provided.
 * It fetches area data using the `GET_AREAS_EXTENDED` query and conditionally renders either the `EditBase`
 * or `Create` component. The form content is rendered once the required data is available.
 *
 * @param props - The props for the UpsertArea component.
 * @param props.record - The initial record data for the Area (optional).
 * @param props.id - The ID of the Area to edit (if editing).
 * @param props.electionEventId - The ID of the election event associated with the Area.
 * @param props.close - Callback function to close the form/modal.
 *
 * @returns A React element that renders the Area form for creation or editing, or null while loading.
 */
export const UpsertArea: React.FC<UpsertAreaProps> = (props) => {
    const {record, id, electionEventId, close} = props

    const [renderUI, setRenderUI] = useState(false)

    const {data: areas} = useQuery(GET_AREAS_EXTENDED, {
        variables: {
            electionEventId,
            areaId: id,
        },
    })

    useEffect(() => {
        if (areas || record) {
            setRenderUI(true)
        }
    }, [areas, record])

    if (renderUI) {
        return (
            <>
                {id ? (
                    <EditBase
                        id={id}
                        resource="sequent_backend_area"
                        mutationMode="pessimistic"
                        redirect={false}
                    >
                        <PageHeaderStyles.Wrapper>
                            <FormContent
                                record={record}
                                id={id}
                                electionEventId={electionEventId}
                                close={close}
                            />
                        </PageHeaderStyles.Wrapper>
                    </EditBase>
                ) : (
                    <Create resource="sequent_backend_area" redirect={false}>
                        <PageHeaderStyles.Wrapper>
                            <FormContent
                                record={record}
                                id={id}
                                electionEventId={electionEventId}
                                close={close}
                            />
                        </PageHeaderStyles.Wrapper>
                    </Create>
                )}
            </>
        )
    } else {
        return null
    }
}
