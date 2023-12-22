// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import {SimpleForm, TextInput, Create, useRefresh, useNotify} from "react-admin"
import {Sequent_Backend_Election_Event} from "../../gql/graphql"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {Tab, Tabs} from "@mui/material"

interface CreateAreaProps {
    record: Sequent_Backend_Election_Event
    close?: () => void
}

export const CreateSupportMaterial: React.FC<CreateAreaProps> = (props) => {
    const {record, close} = props
    const refresh = useRefresh()
    const notify = useNotify()
    const {t} = useTranslation()
    const [valueMaterials, setValueMaterials] = useState(0)
    const [electionEvent, setElectionEvent] = useState<Sequent_Backend_Election_Event | undefined>()

    useEffect(() => {
        if (record) {
            console.log("record", record)
            setElectionEvent(record)
        }
    }, [record])

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

    const handleChangeMaterials = (event: React.SyntheticEvent, newValue: number) => {
        setValueMaterials(newValue)
    }

    const renderTabs = (parsedValue: any, type: string = "general") => {
        let tabNodes = []
        for (const lang in parsedValue?.enabled_languages) {
            if (parsedValue?.enabled_languages[lang]) {
                tabNodes.push(<Tab key={lang} label={t(`common.language.${lang}`)} id={lang}></Tab>)
            }
        }

        // reset actived tab to first tab if only one
        if (tabNodes.length === 1) {
            setValueMaterials(0)
        }

        return tabNodes
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

                    <Tabs value={valueMaterials} onChange={handleChangeMaterials}>
                        {renderTabs(electionEvent, "materials")}
                    </Tabs>
                    {/* {renderTabContentMaterials(parsedValue)} */}

                    {/* <TextInput
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
                    /> */}
                </SimpleForm>
            </PageHeaderStyles.Wrapper>
        </Create>
    )
}
