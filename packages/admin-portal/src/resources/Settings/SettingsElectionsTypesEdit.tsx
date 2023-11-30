// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Button, CircularProgress, Menu, MenuItem, Typography} from "@mui/material"
import React, {useState} from "react"
import {
    Edit,
    EditBase,
    Identifier,
    ReferenceField,
    ReferenceManyField,
    SaveButton,
    SimpleForm,
    TextField,
    TextInput,
    useNotify,
    useRecordContext,
    useRedirect,
    useRefresh,
} from "react-admin"
import {JsonInput} from "react-admin-json-view"
import {ChipList} from "../../components/ChipList"
import {CreateScheduledEventMutation, Sequent_Backend_Area} from "../../gql/graphql"
import {Link} from "react-router-dom"
import {IconButton} from "@sequentech/ui-essentials"
import {faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import {useMutation} from "@apollo/client"
import {CREATE_SCHEDULED_EVENT} from "../../queries/CreateScheduledEvent"
import {ScheduledEventType} from "../../services/ScheduledEvent"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"

interface EditProps {
    id?: Identifier | undefined
    close?: () => void
}

export const SettingselectionsTypesEdit: React.FC<EditProps> = (props) => {
    const {id, close} = props
    const refresh = useRefresh()
    const notify = useNotify()
    const {t} = useTranslation()

    const onSuccess = async (res: any) => {
        refresh()
        notify("Election Types Updated", {type: "success"})

        if (close) {
            close()
        }
    }

    const onError = async (res: any) => {
        refresh()
        notify("Could not Election Types", {type: "error"})

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
                    <PageHeaderStyles.Title>{t("areas.common.title")}</PageHeaderStyles.Title>
                    <PageHeaderStyles.SubTitle>
                        {t("areas.common.subTitle")}
                    </PageHeaderStyles.SubTitle>

                    <TextInput source="name" />
                </SimpleForm>
            </PageHeaderStyles.Wrapper>
        </Edit>
    )
}
