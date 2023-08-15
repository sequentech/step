// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import { useMutation } from "@apollo/client"
import React from "react"
import { CreateElectionEventMutation } from "../../gql/graphql"
import {v4} from "uuid"
import { ArrayInput, ReferenceInput, SelectInput, SimpleForm, SimpleFormIterator, TextInput } from "react-admin"
import { INSERT_ELECTION_EVENT } from "../../queries/InsertElectionEvent"

interface IElectionSubmit {
    description: string
    name: string
}

interface IElectionEventSubmit {
    name: string
    description: string
    elections: Array<IElectionSubmit>
    encryption_protocol: string
    id: string
    tenant_id: string
}

export const CreateElectionList: React.FC = () => {
    const [insertElectionEvent] = useMutation<CreateElectionEventMutation>(INSERT_ELECTION_EVENT)
    const postDefaultValues = () => ({id: v4()})

    const handleSubmit = async (values: any) => {
        const {elections, ...electionSubmit} = values as IElectionEventSubmit
        await insertElectionEvent({
            variables: {
                elections: elections.map((e) => ({
                    election_event_id: electionSubmit.id,
                    tenant_id: electionSubmit.tenant_id,
                    ...e,
                })),
                electionEvent: electionSubmit,
            },
        })
        console.log(values)
    }
    return (
        <SimpleForm defaultValues={postDefaultValues} onSubmit={handleSubmit}>
            <TextInput source="description" />
            <TextInput source="name" />
            <SelectInput source="encryption_protocol" choices={[{id: "RSA256", name: "RSA256"}]} />
            <ReferenceInput source="tenant_id" reference="sequent_backend_tenant">
                <SelectInput optionText="username" />
            </ReferenceInput>
            <ArrayInput source="elections">
                <SimpleFormIterator inline>
                    <TextInput source="name" />
                    <TextInput source="description" />
                </SimpleFormIterator>
            </ArrayInput>
        </SimpleForm>
    )
}