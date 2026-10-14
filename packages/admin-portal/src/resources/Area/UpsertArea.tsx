// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {useLocation, useParams, useSearchParams} from "react-router-dom"
import {Create, Identifier, EditBase, useRecordContext} from "react-admin"
import {Sequent_Backend_Election_Event} from "../../gql/graphql"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {FormContent} from "./FormContent"
import {IAreaPresentation} from "@sequentech/ui-core"

export interface UpsertAreaProps {
    record?: Sequent_Backend_Election_Event
    id?: Identifier | undefined
    electionEventId?: Identifier
    close?: () => void
    area_presentation?: IAreaPresentation
    weightedVotingForAreas?: boolean
}

/**
 * Wrapper component that accesses the record context to pass presentation data
 */
const FormContentWrapper: React.FC<UpsertAreaProps> = (props) => {
    const area_record = useRecordContext()
    if (!area_record) return null
    return (
        <FormContent
            {...props}
            electionEventId={props.electionEventId ?? area_record.election_event_id}
            area_presentation={area_record.presentation}
        />
    )
}

/** Supports both event drawers and standalone area routes. */
export const UpsertArea: React.FC<UpsertAreaProps> = (props) => {
    const {id: routeId} = useParams()
    const [searchParams] = useSearchParams()
    const {pathname} = useLocation()
    const id = props.id ?? (pathname.startsWith("/sequent_backend_area/") ? routeId : undefined)
    const electionEventId =
        props.electionEventId ?? searchParams.get("electionEventId") ?? undefined
    const formProps = {...props, id, electionEventId}

    return id ? (
        <EditBase
            id={id}
            resource="sequent_backend_area"
            mutationMode="pessimistic"
            redirect={false}
        >
            <PageHeaderStyles.Wrapper>
                <FormContentWrapper {...formProps} />
            </PageHeaderStyles.Wrapper>
        </EditBase>
    ) : (
        <Create resource="sequent_backend_area" redirect={false}>
            <PageHeaderStyles.Wrapper>
                <FormContent {...formProps} />
            </PageHeaderStyles.Wrapper>
        </Create>
    )
}
