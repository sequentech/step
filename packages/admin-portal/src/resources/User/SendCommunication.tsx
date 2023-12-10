// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, { useState } from "react"
import {List, SaveButton, SimpleForm, useListContext, useNotify, useRefresh} from "react-admin"
import {SubmitHandler} from "react-hook-form"
import MailIcon from '@mui/icons-material/Mail'
import {useTenantStore} from "@/providers/TenantContextProvider"
import { PageHeaderStyles } from "@/components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import { FormStyles } from "@/components/styles/FormStyles"

interface SendCommunicationProps {
    id?: string
    electionEventId?: string
    close?: () => void
}

export const SendCommunication: React.FC<SendCommunicationProps> = ({
    id, close, electionEventId
}) => {
    const {data, isLoading} = useListContext()
    const [tenantId] = useTenantStore()
    const [communication, setCommunication] = useState({
        email: {
            subject: "Foo",
        },
    })
    //const [sendCommunication] = useMutation<SendCommunicationMutationVariables>(SEND_COMMUNICATION)
    const notify = useNotify()
    const refresh = useRefresh()
    const {t} = useTranslation()


    const onSubmit: SubmitHandler<any> = async () => {
        console.log("sending notification")
    }

    const handleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {name, value} = e.target
        setCommunication({...communication, [name]: value})
    }

    return (
        <PageHeaderStyles.Wrapper>
            <SimpleForm
                toolbar={
                    <SaveButton 
                        icon={<MailIcon />}
                        label={t("sendCommunication.sendButton")}
                        alwaysEnable
                    />
                }
                record={communication}
                onSubmit={onSubmit}
                sanitizeEmptyValues
            >
                <PageHeaderStyles.Title>
                    {t(`sendCommunication.title`)}
                </PageHeaderStyles.Title>
                <PageHeaderStyles.SubTitle>
                    {t(`sendCommunication.subtitle`)}
                </PageHeaderStyles.SubTitle>

                <FormStyles.TextInput
                    label={t("sendCommunication.email.subject")}
                    source="email.subject"
                    onChange={handleChange}
                />
            </SimpleForm>
        </PageHeaderStyles.Wrapper>
    )
}
