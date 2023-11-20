// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Typography} from "@mui/material"
import React from "react"
import {SimpleForm, TextInput, Create} from "react-admin"
import {useMutation} from "@apollo/client"
import { INSERT_TENANT } from "../../queries/InsertTenant"
import { InsertTenantMutation } from "../../gql/graphql"
import {
    FieldValues,
    SubmitHandler,
} from 'react-hook-form'

export const CreateTenant: React.FC = () => {
    const [createTenant] = useMutation<InsertTenantMutation>(INSERT_TENANT)

    const onSubmit: SubmitHandler<FieldValues> = ({slug}) => {
        createTenant({
            variables: {
                slug,
            }
        })
    }
    return (
        <Create>
            <SimpleForm onSubmit={onSubmit}>
                <Typography variant="h4">Customer</Typography>
                <Typography variant="body2">Customer creation</Typography>
                <TextInput source="slug" />
            </SimpleForm>
        </Create>
    )
}
