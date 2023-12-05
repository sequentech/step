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
    SaveButton,
    useGetList,
    CheckboxGroupInput,
} from "react-admin"
import {JsonInput} from "react-admin-json-view"
import {Sequent_Backend_Area, Sequent_Backend_Election_Event} from "../../gql/graphql"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import { useTenantStore } from '@/providers/TenantContextProvider'

interface CreateTallyProps {
    record: Sequent_Backend_Election_Event
    close?: () => void
}

export const CreateTally: React.FC<CreateTallyProps> = (props) => {
    const {record, close} = props
    const refresh = useRefresh()
    const notify = useNotify()
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()

    const {data: elections} = useGetList("sequent_backend_election", {
        pagination: {page: 1, perPage: 9999},
        filter: {election_event_id: record?.id, tenant_id: record?.tenant_id},
    })

    const {data: trustees} = useGetList("sequent_backend_trustee", {
        pagination: {page: 1, perPage: 9999},
        filter: {tenant_id: tenantId},
    })

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
            resource="sequent_backend_tally_session"
            mutationOptions={{onSuccess, onError}}
            redirect={false}
        >
            <PageHeaderStyles.Wrapper>
                <SimpleForm toolbar={<SaveButton alwaysEnable />}>
                    <PageHeaderStyles.Title>{t("tally.common.title")}</PageHeaderStyles.Title>
                    <PageHeaderStyles.SubTitle>
                        {t("tally.common.subTitle")}
                    </PageHeaderStyles.SubTitle>

                    {/* <TextInput source="name" /> */}
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
{/* 
                    {trustees ? (
                        <CheckboxGroupInput
                            label={t("electionEventScreen.tally.trustees")}
                            source="trustee_ids"
                            choices={trustees}
                            optionText="name"
                            optionValue="id"
                            row={false}
                        />
                    ) : null} */}

                    {elections ? (
                        <CheckboxGroupInput
                            label={t("electionEventScreen.tally.elections")}
                            source="election_ids"
                            choices={elections}
                            optionText="name"
                            optionValue="id"
                            row={false}
                        />
                    ) : null}
                </SimpleForm>
            </PageHeaderStyles.Wrapper>
        </Create>
    )
}
