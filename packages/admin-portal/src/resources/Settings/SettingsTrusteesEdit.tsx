// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"

import {useTranslation} from "react-i18next"

import {
    Edit,
    useNotify,
    TextInput,
    SelectInput,
    Identifier,
    SaveButton,
    SimpleForm,
    useRefresh,
    required,
} from "react-admin"

import {ETrusteeModePolicy, getDefaultTrusteeModePolicy} from "@sequentech/ui-core"

import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"

interface EditProps {
    id?: Identifier | undefined
    close?: () => void
}

export const SettingsTrusteesEdit: React.FC<EditProps> = (props) => {
    const {id, close} = props
    const refresh = useRefresh()
    const {t} = useTranslation()

    const trusteeModePolicyChoices = () => {
        return Object.values(ETrusteeModePolicy).map((value) => ({
            id: value,
            name: t(`trusteesSettingsScreen.trusteeModePolicy.options.${value}`),
        }))
    }

    const onSuccess = async () => {
        refresh()

        if (close) {
            close()
        }
    }

    const onError = async () => {
        refresh()

        if (close) {
            close()
        }
    }

    return (
        <Edit
            id={id}
            resource="sequent_backend_trustee"
            mutationMode="pessimistic"
            mutationOptions={{onSuccess, onError}}
            redirect={false}
        >
            <PageHeaderStyles.Wrapper>
                <SimpleForm toolbar={<SaveButton />}>
                    <PageHeaderStyles.Title>
                        {t("trusteesSettingsScreen.edit.title")}
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
                </SimpleForm>
            </PageHeaderStyles.Wrapper>
        </Edit>
    )
}
