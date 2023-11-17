// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Typography} from "@mui/material"
import React from "react"
import {
    SimpleForm,
    TextInput,
    SelectInput,
    ReferenceInput,
    Create,
    FormDataConsumer,
    ReferenceField,
    useRecordContext,
    useRefresh,
    useNotify,
} from "react-admin"
import {JsonInput} from "react-admin-json-view"
import {Sequent_Backend_Area, Sequent_Backend_Election_Event} from "../../gql/graphql"

interface CreateAreaProps {
    record: Sequent_Backend_Election_Event
    close?: () => void
}

export const CreateArea: React.FC<CreateAreaProps> = (props) => {
    const {record, close} = props
    const refresh = useRefresh()
    const notify = useNotify()

    const onSuccess = () => {
        refresh()
        notify("Area created", {type: "success"})
        if (close) {
            close()
        }
    }

    const onError = async (res: any) => {
        refresh()
        notify("Could not create Area", {type: "error"})
        if (close) {
            close()
        }
    }

    return (
        <Create resource="sequent_backend_area" mutationOptions={{onSuccess, onError}} redirect={false}>
            <SimpleForm>
                <Typography variant="h4">Area</Typography>
                <Typography variant="body2">Area configuration</Typography>

                <TextInput source="name" />
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

            {/* <SimpleForm>
                <Typography variant="h4">Area</Typography>
                <Typography variant="body2">Area creation</Typography>
                <TextInput source="name" />
                <TextInput source="description" />
                <TextInput source="type" />
                <ReferenceInput source="tenant_id" reference="sequent_backend_tenant">
                    <SelectInput optionText="username" />
                </ReferenceInput>
                <FormDataConsumer>
                    {({formData}) => (
                        <ReferenceInput
                            source="election_event_id"
                            reference="sequent_backend_election_event"
                            filter={{tenant_id: formData.tenant_id}}
                        >
                            <SelectInput optionText="name" />
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
            </SimpleForm> */}
        </Create>
    )
}
