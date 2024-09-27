// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect} from "react"

import {useTranslation} from "react-i18next"

import {
    useNotify,
    TextInput,
    Identifier,
    SaveButton,
    SimpleForm,
    useRefresh,
    useUpdate,
    useGetOne,
    SelectInput,
    DateTimeInput,
    RaRecord,
    Toolbar,
} from "react-admin"

import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {Sequent_Backend_Tenant} from "@/gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ISchedule, ScheduledEvents} from "./constants"
import {Typography} from "@mui/material"

interface EditProps {
    id?: Identifier | undefined
    close?: () => void
}

export const SettingsSchedulesEdit: React.FC<EditProps> = (props) => {
    const {id, close} = props
    const refresh = useRefresh()
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()
    const notify = useNotify()

    const [scheduleData, setScheduleData] = React.useState<RaRecord<Identifier> | undefined>()

    const [update] = useUpdate("sequent_backend_tenant")
    const {data} = useGetOne<Sequent_Backend_Tenant>("sequent_backend_tenant", {
        id: tenantId,
    })

    useEffect(() => {
        const schedule = data?.settings?.schedules.find((s: ISchedule) => s.id === id)
        setScheduleData(schedule)
    }, [id, data])

    const handleSubmit = (newItem: any) => {
        const oldSettings = data?.settings?.schedules.filter((s: ISchedule) => s.id !== id)

        const sendData = {
            ...data,
            settings: {
                ...data?.settings,
                schedules: [...oldSettings, newItem],
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
        <SimpleForm
            onSubmit={handleSubmit}
            record={scheduleData}
            toolbar={
                <Toolbar>
                    <SaveButton />
                </Toolbar>
            }
        >
            <PageHeaderStyles.Title>{t("scheduleScreen.create.title")}</PageHeaderStyles.Title>
            <Typography variant="body2" color="textSecondary">
                {t("scheduleScreen.create.selectSchedule")}
            </Typography>
            <SelectInput
                source="name"
                choices={Object.entries(ScheduledEvents).map(([_key, value]) => ({
                    id: t(`scheduleScreen.eventTypes.${value}`),
                    name: t(`scheduleScreen.eventTypes.${value}`),
                }))}
            />
            <TextInput source="name" />
            <DateTimeInput source="date" />
        </SimpleForm>
    )
}
