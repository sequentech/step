// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useElectionEventStore} from "@/providers/ElectionEventContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {Box, Typography, styled} from "@mui/material"
import {isNull} from "@sequentech/ui-essentials"
import React, {useEffect, useState} from "react"
import {
    BooleanInput,
    Create,
    FormDataConsumer,
    NumberInput,
    ReferenceInput,
    SelectInput,
    SimpleForm,
    TextInput,
    useGetOne,
    useNotify,
    useRefresh,
} from "react-admin"
import {JsonInput} from "react-admin-json-view"
import {useTranslation} from "react-i18next"
import {useNavigate} from "react-router"
import { useSearchParams } from 'react-router-dom'

const Hidden = styled(Box)`
    display: none;
`

interface IElectionSubmit {
    description: string
    name: string
    tenant_id: string
    election_event_id: string
}

export const CreateElection: React.FC = () => {
    const [tenantId] = useTenantStore()
    const [searchParams] = useSearchParams()
    
    const electionEventId = searchParams.get("electionEventId")


    return (
        <Create>
            <SimpleForm>
                <Typography variant="h4">Create Election</Typography>
                <TextInput source="name" />
                <TextInput source="description" />
                {/* <Hidden> */}
                <BooleanInput source="is_consolidated_ballot_encoding" />
                <BooleanInput source="spoil_ballot_option" />
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
                            <SelectInput
                                optionText="name"
                                defaultValue={electionEventId}
                            />
                        </ReferenceInput>
                    )}
                </FormDataConsumer>
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
                    source="dates"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
                <JsonInput
                    source="status"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
                <TextInput source="eml" />
                <NumberInput source="num_allowed_revotes" />
                {/* </Hidden> */}
            </SimpleForm>
        </Create>
    )
}
