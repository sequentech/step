// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, { useEffect } from "react"

import {useTranslation} from "react-i18next"

import {
    Edit,
    useNotify,
    TextInput,
    Identifier,
    SaveButton,
    SimpleForm,
    useRefresh,
    useUpdate,
    useGetOne,
} from "react-admin"

import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {Sequent_Backend_Tenant} from "@/gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"
import { ISchedule } from './constants'

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

    const [update] = useUpdate("sequent_backend_tenant")
    const {data} = useGetOne<Sequent_Backend_Tenant>("sequent_backend_tenant", {
        id: tenantId,
    })

    useEffect(() => {
        console.log("record edit data", data)
        const schedule = data?.settings?.schedules.find((s: ISchedule) => s.id === id)
        console.log("record edit schedule", schedule)
    }, [id, data])

    // const onSuccess = async () => {
    //     refresh()

    //     if (close) {
    //         close()
    //     }
    // }

    // const onError = async () => {
    //     refresh()

    //     if (close) {
    //         close()
    //     }
    // }

    return (
        <Edit
            id={id}
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
