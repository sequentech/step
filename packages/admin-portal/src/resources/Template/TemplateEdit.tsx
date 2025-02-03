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
import {is_communication_template_type} from "@/lib/helpers"

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

        // Call the UpdateTemplate mutation with the updated values
        // and the original ID and tenant ID
        const {data: updated, errors} = await UpdateTemplate({
            variables: {
                // The ID of the template to update
                id: id,
                // The tenant ID of the template
                tenantId: tenantId,
                // The fields to update and their new values
                set: {
                    // The alias of the template
                    alias: aliasValue,
                    // The annotations of the template
                    annotations: data.annotations,
                    // The communication method of the template
                    communication_method: data.communication_method,
                    // The created at timestamp of the template
                    created_at: data.created_at,
                    // The created by user ID of the template
                    created_by: data.created_by,
                    // The labels of the template
                    labels: data.labels,
                    // The template data
                    template: {
                        // Copy the original template data
                        ...data.template,
                        // Update the alias of the template
                        alias: aliasValue,
                    },
                    // The tenant ID of the template
                    tenant_id: data.tenant_id,
                    // The type of the template
                    type: data.type,
                    // The updated at timestamp of the template
                    updated_at: data.updated_at,
                    // The active status of the template
                    is_active: data.is_active,
                    // Whether the template is a communication template
                    // based on its type
                    is_communication: is_communication_template_type(data.type),
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
