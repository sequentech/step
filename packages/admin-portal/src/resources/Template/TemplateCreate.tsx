// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {CreateBase, SimpleForm, useNotify} from "react-admin"
import {FieldValues, SubmitHandler, useForm} from "react-hook-form"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {useMutation} from "@apollo/client"
import {ITemplateMethod} from "@/types/templates"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {INSERT_TEMPLATE} from "@/queries/InsertTemplate"
import {TemplateFormContent} from "./TemplateFormContent"
import {is_communication_template_type} from "@/lib/helpers"

type TTemplateCreate = {
    close?: () => void
}

export const TemplateCreate: React.FC<TTemplateCreate> = ({close}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const notify = useNotify()
    const [createTemplate] = useMutation(INSERT_TEMPLATE)

    const onSubmit: SubmitHandler<FieldValues> = async (data) => {
        data.communication_method = ITemplateMethod.EMAIL

        // Create the template in the database
        // We use the createTemplate mutation that is defined in the INSERT_TEMPLATE query
        // The mutation takes the template object as an argument
        // The template object contains the template data
        const {data: created, errors} = await createTemplate({
            variables: {
                object: {
                    // The alias of the template
                    alias: data.template.alias,
                    // The tenant id of the template
                    tenant_id: tenantId,
                    // The type of the template
                    type: data.type,
                    // The communication method of the template
                    communication_method: data.communication_method,
                    // Whether the template is active or not
                    is_active: data.is_active,
                    // Whether the template is a communication template or not
                    is_communication: is_communication_template_type(data.type),
                    // The template data
                    template: {
                        ...data.template,
                    },
                },
            },
        })

        if (created) {
            notify(t("template.create.success"), {type: "success"})
        }

        if (errors) {
            notify(t("template.create.error"), {type: "error"})
        }

        close?.()
    }

    return (
        <CreateBase resource="sequent_backend_template" redirect={false}>
            <PageHeaderStyles.Wrapper>
                <SimpleForm onSubmit={onSubmit}>
                    <TemplateFormContent isTemplateEdit={false} />
                </SimpleForm>
            </PageHeaderStyles.Wrapper>
        </CreateBase>
    )
}
