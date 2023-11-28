// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import { useElectionStore } from '@/providers/ElectionContextProvider'
import { useElectionEventStore } from '@/providers/ElectionEventContextProvider'
import { useTenantStore } from '@/providers/TenantContextProvider'
import {Box, Typography, styled} from "@mui/material"
import React from "react"
import {
    BooleanInput,
    SimpleForm,
    TextInput,
    NumberInput,
    SelectInput,
    ReferenceInput,
    Create,
    FormDataConsumer,
} from "react-admin"
import {JsonInput} from "react-admin-json-view"
import { useSearchParams } from 'react-router-dom'

const Hidden = styled(Box)`
    display: none;
`

interface IContestSubmit {
    description: string
    name: string
    tenant_id: string
    election_event_id: string,
    election_id: string,
}

export const CreateContest: React.FC = () => {
    const [tenantId] = useTenantStore()
    const [searchParams] = useSearchParams()
    
    const electionEventId = searchParams.get("electionEventId")
    const electionId = searchParams.get("electionId")
    
    
    return (
        <Create>
            <SimpleForm>
                <Typography variant="h4">Contest</Typography>
                <Typography variant="body2">Contest configuration</Typography>
                <TextInput source="name" />
                <TextInput source="description" />

                {/* <Hidden> */}
                <BooleanInput source="is_acclaimed" />
                <BooleanInput source="is_active" />
                <NumberInput source="min_votes" />
                <NumberInput source="max_votes" />
                <SelectInput
                    source="voting_type"
                    choices={[{id: "first-past-the-post", name: "First Past The Post"}]}
                />
                <SelectInput
                    source="counting_algorithm"
                    choices={[{id: "plurality-at-large", name: "Plurality At Large"}]}
                />
                <BooleanInput source="is_encrypted" />
                <ReferenceInput source="tenant_id" reference="sequent_backend_tenant">
                    <SelectInput optionText="slug" defaultValue={tenantId} />
                </ReferenceInput>
                <FormDataConsumer>
                    {({formData}) => (
                        <ReferenceInput
                            source="election_event_id"
                            reference="sequent_backend_election_event"
                            filter={{id: electionEventId}}
                        >
                            <SelectInput optionText="name" defaultValue={electionEventId} />
                        </ReferenceInput>
                    )}
                </FormDataConsumer>
                <FormDataConsumer>
                    {({formData}) => (
                        <ReferenceInput
                            source="election_id"
                            reference="sequent_backend_election"
                            filter={{id: electionId}}
                        >
                            <SelectInput optionText="name" defaultValue={electionId} />
                        </ReferenceInput>
                    )}
                </FormDataConsumer>

                {/* <ReferenceInput source="tenant_id" reference="sequent_backend_tenant">
                        <SelectInput optionText="username" />
                    </ReferenceInput>
                    <FormDataConsumer>
                        {({formData}) => (
                            <>
                                <ReferenceInput
                                    source="election_event_id"
                                    reference="sequent_backend_election_event"
                                    filter={{tenant_id: formData.tenant_id}}
                                >
                                    <SelectInput optionText="name" />
                                </ReferenceInput>
                                <ReferenceInput
                                    source="election_id"
                                    reference="sequent_backend_election"
                                    filter={{
                                        tenant_id: formData.tenant_id,
                                        election_event_id: formData.election_event_id,
                                    }}
                                >
                                    <SelectInput optionText="name" />
                                </ReferenceInput>
                            </>
                        )}
                    </FormDataConsumer> */}
                <JsonInput
                    source="labels"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
                <JsonInput
                    source="labels"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
                <JsonInput
                    source="annotations"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
                <JsonInput
                    source="presentation"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
                <JsonInput
                    source="tally_configuration"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
                <JsonInput
                    source="conditions"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
                {/* </Hidden> */}
            </SimpleForm>
        </Create>
    )
}
