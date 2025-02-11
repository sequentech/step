// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"

import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    FormControl,
    FormGroup,
    FormLabel,
} from "@mui/material"

import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {
    BooleanInput,
    EditBase,
    FormDataConsumer,
    Identifier,
    RaRecord,
    RecordContext,
    SaveButton,
    SelectInput,
    SimpleForm,
    required,
    useNotify,
    useRefresh,
} from "react-admin"

import {FieldValues, SubmitHandler} from "react-hook-form"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {UPDATE_TEMPLATE} from "@/queries/UpdateTemplate"
import {TemplateFormContent} from "./TemplateFormContent"

type TTemplateEdit = {
    id?: Identifier | undefined
    close?: () => void
}

export const TemplateEdit: React.FC<TTemplateEdit> = (props) => {
    const {id, close} = props
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const notify = useNotify()
    const refresh = useRefresh()
    const [UpdateTemplate] = useMutation(UPDATE_TEMPLATE)
    const [saveEnabled, setSaveEnabled] = React.useState(false)
    const onSubmit: SubmitHandler<FieldValues> = async (data) => {
        const aliasValue = data.template.alias

        const {data: updated, errors} = await UpdateTemplate({
            variables: {
                id: id,
                tenantId: tenantId,
                set: {
                    alias: aliasValue,
                    annotations: data.annotations,
                    communication_method: data.communication_method,
                    created_at: data.created_at,
                    created_by: data.created_by,
                    labels: data.labels,
                    template: {
                        ...data.template,
                        alias: aliasValue,
                    },
                    tenant_id: data.tenant_id,
                    type: data.type,
                    updated_at: data.updated_at,
                },
            },
        })

        if (updated) {
            notify(t("template.update.success"), {type: "success"})
        }

        if (errors) {
            notify(t("template.update.error"), {type: "error"})
        }

        close?.()
    }

    const onSuccess = async (res: any) => {
        refresh()
        notify("Area updated", {type: "success"})
        if (close) {
            setTimeout(() => {
                close()
            }, 400)
        }
    }

    const onError = async (res: any) => {
        console.log("onError :>> ", res)

        refresh()
        notify("Could not update Area", {type: "error"})
        if (close) {
            setTimeout(() => {
                close()
            }, 400)
        }
    }

    return (
        <EditBase
            id={id}
            resource="sequent_backend_template"
            mutationMode="pessimistic"
            mutationOptions={{onSuccess, onError}}
            redirect={false}
        >
            <PageHeaderStyles.Wrapper>
                <SimpleForm onSubmit={onSubmit} toolbar={<SaveButton alwaysEnable={saveEnabled} />}>
                    <TemplateFormContent
                        isTemplateEdit={true}
                        onFormChanged={() => setSaveEnabled(true)}
                    />
                </SimpleForm>
            </PageHeaderStyles.Wrapper>
        </EditBase>
    )
}
