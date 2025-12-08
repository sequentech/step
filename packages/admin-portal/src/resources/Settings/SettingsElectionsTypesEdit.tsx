// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"

import {useTranslation} from "react-i18next"

import {
    Edit,
    useNotify,
    TextInput,
    Identifier,
    SaveButton,
    SimpleForm,
    useRefresh,
} from "react-admin"

import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"

interface EditProps {
    id?: Identifier | undefined
    close?: () => void
}

export const SettingselectionsTypesEdit: React.FC<EditProps> = (props) => {
    const {id, close} = props
    const refresh = useRefresh()
    const {t} = useTranslation()

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
            resource="sequent_backend_election_type"
            mutationMode="pessimistic"
            mutationOptions={{onSuccess, onError}}
            redirect={false}
        >
            <PageHeaderStyles.Wrapper>
                <SimpleForm toolbar={<SaveButton />}>
                    <PageHeaderStyles.Title>
                        {t("electionTypeScreen.edit.title")}
                    </PageHeaderStyles.Title>

                    <TextInput source="name" />
                </SimpleForm>
            </PageHeaderStyles.Wrapper>
        </Edit>
    )
}
