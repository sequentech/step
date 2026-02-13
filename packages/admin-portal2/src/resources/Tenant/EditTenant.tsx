// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Typography} from "@mui/material"
import React from "react"
import {BooleanInput, Edit, SimpleForm, TextField, TextInput} from "react-admin"
import {JsonInput} from "react-admin-json-view"
import {ListTenant} from "./ListTenant"

const TenantForm: React.FC = () => {
    return (
        <SimpleForm>
            <Typography variant="h4">Customer</Typography>
            <Typography variant="body2">Customer configuration</Typography>
            <Typography variant="h5">ID</Typography>
            <TextField source="id" />
            <TextInput source="slug" />
            <BooleanInput source="is_active" />
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
    )
}

export const EditTenant: React.FC = () => {
    return (
        <ListTenant
            aside={
                <Edit sx={{flexGrow: 2, width: "50%"}}>
                    <TenantForm />
                </Edit>
            }
        />
    )
}
