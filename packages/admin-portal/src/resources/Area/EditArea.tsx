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
import {ListArea} from "./ListArea"
import {JsonInput} from "react-admin-json-view"
import {ChipList} from "../../components/ChipList"
import {CreateScheduledEventMutation, Sequent_Backend_Area} from "../../gql/graphql"
import {Link} from "react-router-dom"
import {IconButton} from "@sequentech/ui-essentials"
import {faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import {useTenantStore} from "../../components/CustomMenu"
import {useMutation} from "@apollo/client"
import {CREATE_SCHEDULED_EVENT} from "../../queries/CreateScheduledEvent"
import {ScheduledEventType} from "../../services/ScheduledEvent"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"

interface EditAreaProps {
    id?: Identifier | undefined
    close?: () => void
}

export const EditArea: React.FC<EditAreaProps> = (props) => {
    const {id, close} = props
    const refresh = useRefresh()
    const notify = useNotify()
    const {t} = useTranslation()

    const onSuccess = async (res: any) => {
        refresh()
        notify("Area updated", {type: "success"})
        if (close) {
            close()
        }
    }

    const onError = async (res: any) => {
        refresh()
        notify("Could not update Area", {type: "error"})
        if (close) {
            close()
        }
    }

    return (
        <Edit
            id={id}
            resource="sequent_backend_area"
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
