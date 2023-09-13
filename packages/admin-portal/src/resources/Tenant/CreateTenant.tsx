// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Typography} from "@mui/material"
import React from "react"
import {SimpleForm, TextInput, Create, BooleanInput, ReferenceInput, SelectInput} from "react-admin"
import {JsonInput} from "react-admin-json-view"

export const CreateTenant: React.FC = () => {
    return (
        <Create>
            <SimpleForm>
                <Typography variant="h4">Customer</Typography>
                <Typography variant="body2">Customer creation</Typography>
                <TextInput source="username" />
                <BooleanInput source="is_active" />
                <ReferenceInput source="tenant_id" reference="sequent_backend_tenant">
                    <SelectInput optionText="username" />
                </ReferenceInput>
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
            </SimpleForm>
        </Create>
    )
}
