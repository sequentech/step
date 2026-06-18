// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"

import {Box} from "@mui/material"
import {Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"

import {
    Create,
    ReferenceInput,
    SelectInput,
    SimpleForm,
    TextInput,
    useRefresh,
    useNotify,
    required,
} from "react-admin"

import {ETrusteeModePolicy, getDefaultTrusteeModePolicy} from "@sequentech/ui-core"

import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"

import {useTenantStore} from "@/providers/TenantContextProvider"

const Hidden = styled(Box)`
    display: none;
`

interface CreateProps {
    close?: () => void
}

export const SettingsTrusteesCreate: React.FC<CreateProps> = (props) => {
    const {close} = props
    const refresh = useRefresh()
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()

    const trusteeModePolicyChoices = () => {
        return Object.values(ETrusteeModePolicy).map((value) => ({
            id: value,
            name: t(`trusteesSettingsScreen.trusteeModePolicy.options.${value}`),
        }))
    }

    const onSuccess = () => {
        refresh()
        if (close) {
            close()
        }
    }

    const onError = async (res: any) => {
        refresh()
        if (close) {
            close()
        }
    }

    return (
        <Create
            mutationOptions={{
                onSuccess: close ? onSuccess : undefined,
                onError: close ? onError : undefined,
            }}
            redirect={close ? false : (resource: any, id: any, data: any): string => "settings"}
            resource="sequent_backend_trustee"
        >
            <SimpleForm>
                <PageHeaderStyles.Title>
                    {t("trusteesSettingsScreen.create.title")}
                </PageHeaderStyles.Title>
                <TextInput source="name" />
                <TextInput source="public_key" />
                <SelectInput
                    source="annotations.trustee_mode_policy"
                    choices={trusteeModePolicyChoices()}
                    label={String(t("trusteesSettingsScreen.trusteeModePolicy.label"))}
                    defaultValue={getDefaultTrusteeModePolicy()}
                    validate={required()}
                />

                <Hidden>
                    <ReferenceInput source="tenant_id" reference="sequent_backend_tenant">
                        <SelectInput optionText="slug" defaultValue={tenantId} />
                    </ReferenceInput>
                </Hidden>
            </SimpleForm>
        </Create>
    )
}
