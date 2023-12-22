// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {SimpleForm, TextInput, Create, useRefresh, useNotify} from "react-admin"
import {Sequent_Backend_Election_Event} from "../../gql/graphql"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"

interface CreateAreaProps {
    record: Sequent_Backend_Election_Event
    close?: () => void
}

export const CreateArea: React.FC<CreateAreaProps> = (props) => {
    const {record, close} = props
    const refresh = useRefresh()
    const notify = useNotify()
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
            resource="sequent_backend_area"
            mutationOptions={{onSuccess, onError}}
            redirect={false}
        >
            <PageHeaderStyles.Wrapper>
                <SimpleForm>
                    <PageHeaderStyles.Title>{t("areas.common.title")}</PageHeaderStyles.Title>
                    <PageHeaderStyles.SubTitle>
                        {t("areas.common.subTitle")}
                    </PageHeaderStyles.SubTitle>

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
            </PageHeaderStyles.Wrapper>
        </Create>
    )
}
