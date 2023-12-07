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
    useGetOne,
} from "react-admin"
import {JsonInput} from "react-admin-json-view"
import {
    Sequent_Backend_Area,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Keys_Ceremony,
    Sequent_Backend_Trustee,
} from "../../gql/graphql"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"

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

    const {data: keyCeremony} = useGetList<Sequent_Backend_Keys_Ceremony>(
        "sequent_backend_keys_ceremony",
        {
            pagination: {page: 1, perPage: 9999},
            filter: {election_event_id: record?.id, tenant_id: record?.tenant_id},
        }
    )

    console.log("keyCeremony", keyCeremony)

    const {data: elections} = useGetList("sequent_backend_election", {
        pagination: {page: 1, perPage: 9999},
        filter: {election_event_id: record?.id, tenant_id: record?.tenant_id},
    })

    const {data: trustees} = useGetList<Sequent_Backend_Trustee>("sequent_backend_trustee", {
        pagination: {page: 1, perPage: 9999},
        filter: {tenant_id: tenantId},
    })

    const onSuccess = () => {
        refresh()
        notify(t("tally.createTallySuccess"), {type: "success"})
        if (close) {
            close()
        }
    }

    const onError = async (res: any) => {
        refresh()
        notify(t("tally.createTallyError"), {type: "error"})
        if (close) {
            close()
        }
    }

    return (
        <>
            {keyCeremony && elections ? (
                <Create
                    resource="sequent_backend_tally_session"
                    mutationOptions={{onSuccess, onError}}
                    redirect={false}
                >
                    <PageHeaderStyles.Wrapper>
                        <SimpleForm toolbar={<SaveButton alwaysEnable />}>
                            <PageHeaderStyles.Title>
                                {t("tally.common.title")}
                            </PageHeaderStyles.Title>
                            <PageHeaderStyles.SubTitle>
                                {t("tally.common.subTitle")}
                            </PageHeaderStyles.SubTitle>

                            {/* <TextInput source="name" /> */}
                            <SelectInput
                                choices={keyCeremony}
                                label="Key Ceremony"
                                source="keys_ceremony_id"
                                optionText={"id"}
                                optionValue={"id"}
                                defaultValue={keyCeremony[0].id}
                                // style={{display: "none"}}
                            />
                            <TextInput
                                label="Election Event"
                                source="election_event_id"
                                defaultValue={record?.id}
                                // style={{display: "none"}}
                            />
                            <TextInput
                                label="Tenant"
                                source="tenant_id"
                                defaultValue={record?.tenant_id}
                                // style={{display: "none"}}
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
            ) : null}
        </>
    )
}
