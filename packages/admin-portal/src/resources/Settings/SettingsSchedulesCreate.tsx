// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"

import {Typography} from "@mui/material"
import {useTranslation} from "react-i18next"

import {
    Create,
    SelectInput,
    SimpleForm,
    TextInput,
    useRefresh,
    useNotify,
    DateInput,
    useUpdate,
    useGetOne,
} from "react-admin"

import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"

import {useTenantStore} from "@/providers/TenantContextProvider"
import {SCHEDULE_NAMES_LIST} from "./constants"
import {Sequent_Backend_Tenant} from "@/gql/graphql"


interface CreateProps {
    close?: () => void
}

export const SettingsSchedulesCreate: React.FC<CreateProps> = (props) => {
    const {close} = props
    const refresh = useRefresh()
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()
    const notify = useNotify()

    const [update] = useUpdate("sequent_backend_tenant")
    const {data} = useGetOne<Sequent_Backend_Tenant>("sequent_backend_tenant", {
        id: tenantId,
    })

    const handleSubmit = (newItem: any) => {
        
        newItem.id = crypto.randomUUID()
        console.log("record newItem", newItem)

        const sendData = {
            ...data,
            settings: {
                ...data?.settings,
                schedules: [...(data?.settings?.schedules ?? []), newItem],
            },
        }

        update(
            "sequent_backend_tenant",
            {
                id: tenantId,
                data: sendData,
            },
            {
                onSuccess: () => {
                    notify(t("scheduleScreen.createScheduleSuccess"), {type: "success"})
                    refresh()
                    if (close) {
                        close()
                    }
                },
                onError: (error) => {
                    notify(t("scheduleScreen.createScheduleError"), {type: "error"})
                    refresh()
                    if (close) {
                        close()
                    }
                },
            }
        )
    }

    return (
        <Create>
            <SimpleForm onSubmit={handleSubmit}>
                <PageHeaderStyles.Title>{t("scheduleScreen.create.title")}</PageHeaderStyles.Title>
                <Typography variant="body2" color="textSecondary">
                    {t("scheduleScreen.create.selectSchedule")}
                </Typography>
                <SelectInput
                    source="name"
                    choices={SCHEDULE_NAMES_LIST.map((item) => ({
                        id: item,
                        name: t(item),
                    }))}
                />
                <TextInput source="name" />
                <DateInput source="date" />

            </SimpleForm>
        </Create>
    )
}
