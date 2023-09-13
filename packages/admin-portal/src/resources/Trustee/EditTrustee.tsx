// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Typography} from "@mui/material"
import React from "react"
import {BooleanInput, Edit, SimpleForm, TextField, TextInput} from "react-admin"
import {ListTrustee} from "./ListTrustee"
import {JsonInput} from "react-admin-json-view"

const TrusteeForm: React.FC = () => {
    return (
        <SimpleForm>
            <Typography variant="h4">Trustee</Typography>
            <Typography variant="body2">Trustee configuration</Typography>
            <Typography variant="h5">ID</Typography>
            <TextField source="id" />
            <TextInput source="name" />
            <TextInput source="public_key" />
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

export const EditTrustee: React.FC = () => {
    return (
        <ListTrustee
            aside={
                <Edit sx={{flexGrow: 2, width: "50%", flexShrink: 0}}>
                    <TrusteeForm />
                </Edit>
            }
        />
    )
}
