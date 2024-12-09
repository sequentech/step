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

        const {data: created, errors} = await createTemplate({
            variables: {
                object: {
                    alias: data.template.alias,
                    tenant_id: tenantId,
                    type: data.type,
                    communication_method: data.communication_method,
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
