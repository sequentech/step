// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Typography} from "@mui/material"
import React from "react"
import {
    BooleanInput,
    Create,
    FormDataConsumer,
    NumberInput,
    ReferenceInput,
    SelectInput,
    SimpleForm,
    TextInput,
    useRefresh,
    useNotify,
} from "react-admin"
import {styled} from "@mui/material/styles"
import {Box} from "@mui/material"
import { useTenantStore } from '@/providers/TenantContextProvider'
import { useTranslation } from 'react-i18next'
import { Sequent_Backend_Election_Type } from '@/gql/graphql'

const Hidden = styled(Box)`
    display: none;
`

interface CreateProps {
    close?: () => void
}

export const SettingsElectionsTypesCreate: React.FC<CreateProps> = (props) => {
    const {close} = props
    const refresh = useRefresh()
    const notify = useNotify()
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()


    const onSuccess = () => {
        refresh()
        notify(t("areas.createAreaSuccess"), {type: "success"})
        if (close) {
            close()
        }
    }

    const onError = async (res: any) => {
        refresh()
        notify("areas.createAreaError", {type: "error"})
        if (close) {
            close()
        }
    }

    return (
        <Create 
            mutationOptions={{onSuccess: close ? onSuccess : undefined, onError: close ? onError : undefined}} 
            redirect={close ? false : (resource: any, id: any, data: any): string => 'settings'}
        >
            <SimpleForm>
                <Typography variant="h4">Create Election</Typography>
                <TextInput source="name" />


                <Hidden>
                    <ReferenceInput source="tenant_id" reference="sequent_backend_tenant">
                        <SelectInput optionText="slug" defaultValue={tenantId} />
                    </ReferenceInput>
                </Hidden>
            </SimpleForm>
        </Create>
    )
}
