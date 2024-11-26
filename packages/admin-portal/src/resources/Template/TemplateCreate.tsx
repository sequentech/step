// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"

import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    FormControl,
    FormLabel,
    FormGroup,
} from "@mui/material"

import {
    CreateBase,
    SelectInput,
    SimpleForm,
    required,
    useNotify,
    BooleanInput,
    FormDataConsumer,
} from "react-admin"

import {FieldValues, SubmitHandler, useForm} from "react-hook-form"
import {FormStyles} from "@/components/styles/FormStyles"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {useMutation} from "@apollo/client"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {ETemplateType, ITemplateMethod, ISendTemplateBody} from "@/types/templates"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {INSERT_TEMPLATE} from "@/queries/InsertTemplate"
import EmailEditEditor from "@/components/EmailEditEditor"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {GET_USER_TEMPLATE} from "@/queries/GetUserTemplate"
import {IPermissions} from "@/types/keycloak"
import {useFormContext} from "react-hook-form"

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
                    <FormContent />
                </SimpleForm>
            </PageHeaderStyles.Wrapper>
        </CreateBase>
    )
}

const FormContent = () => {
    const {t} = useTranslation()
    const {setValue} = useFormContext()
    const {globalSettings} = useContext(SettingsContext)

    const [expandedGeneral, setExpandedGeneral] = useState(true)
    const [expandedEmail, setExpandedEmail] = useState(false)
    const [expandedSMS, setExpandedSMS] = useState(false)
    const [expandedDocument, setExpandedDocument] = useState(false)
    const [templateHbsData, setTemplateHbsData] = useState<string | undefined>(undefined)

    const [GetUserTemplate] = useMutation(GET_USER_TEMPLATE, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.REPORT_READ,
            },
        },
    })
    const [selectedTemplateType, setSelectedTemplateType] = useState<{
        name: string
        value: ETemplateType
    }>()

    function selectTemplateType(event: any) {
        const choice = event.target
        setSelectedTemplateType(choice)
    }

    const templateTypeChoices = () => {
        return (Object.values(ETemplateType) as ETemplateType[]).map((value) => ({
            id: value,
            name: t(`template.type.${value}`),
        }))
    }

    useEffect(() => {
        const fetchTemplateData = async () => {
            try {
                const currType = selectedTemplateType?.value as ETemplateType
                const {data: templateData, errors} = await GetUserTemplate({
                    variables: {
                        template_type: "statistical_report", //TODO: Adjust the route to return the HBS and email data based on the template *TYPE*
                    },
                })
                setTemplateHbsData(templateData?.get_user_template.template_hbs)
            } catch (error) {
                console.error("Error fetching template data:", error)
            }
        }
        if (selectedTemplateType) {
            fetchTemplateData()
        }
    }, [selectedTemplateType])

    useEffect(() => {
        setValue(
            "template.document",
            templateHbsData || globalSettings.DEFAULT_DOCUMENT["en"] || ""
        )
    }, [templateHbsData])

    return (
        <FormControl fullWidth>
            <ElectionHeaderStyles.Wrapper>
                <PageHeaderStyles.Title>{t("template.create.title")}</PageHeaderStyles.Title>
            </ElectionHeaderStyles.Wrapper>
            <Accordion
                sx={{width: "100%"}}
                expanded={expandedGeneral}
                onChange={() => setExpandedGeneral(!expandedGeneral)}
            >
                <AccordionSummary expandIcon={<ExpandMoreIcon id="election-event-data-general" />}>
                    <ElectionHeaderStyles.AccordionTitle>
                        {t("electionEventScreen.edit.general")}
                    </ElectionHeaderStyles.AccordionTitle>
                </AccordionSummary>
                <AccordionDetails>
                    <FormStyles.TextInput
                        source="template.alias"
                        validate={required()}
                        label={t("template.form.alias")}
                    />

                    <FormStyles.TextInput
                        source="template.name"
                        validate={required()}
                        label={t("template.form.name")}
                    />
                    <SelectInput
                        source="type"
                        label={t("template.form.type")}
                        validate={required()}
                        onChange={selectTemplateType}
                        choices={templateTypeChoices()}
                    />
                </AccordionDetails>
            </Accordion>

            <FormLabel component="legend">{t(`template.chooseMethods`)}</FormLabel>
            <FormGroup
                sx={{
                    display: "flex",
                    flexDirection: "row",
                    gap: "16px",
                }}
            >
                {Object.values(ITemplateMethod).map((method) => (
                    <BooleanInput
                        key={method}
                        source={`template.selected_methods.${method}`}
                        label={t(`template.method.${method.toLowerCase()}`)}
                    />
                ))}
            </FormGroup>
            <FormDataConsumer>
                {({formData}) => (
                    <>
                        {formData.template?.selected_methods?.EMAIL && (
                            <Accordion
                                sx={{width: "100%"}}
                                expanded={expandedEmail}
                                onChange={() => setExpandedEmail(!expandedEmail)}
                            >
                                <AccordionSummary
                                    expandIcon={<ExpandMoreIcon id="template-email-id" />}
                                >
                                    <ElectionHeaderStyles.AccordionTitle>
                                        {t("template.method.email")}
                                    </ElectionHeaderStyles.AccordionTitle>
                                </AccordionSummary>
                                <AccordionDetails>
                                    <EmailEditEditor
                                        sourceSubject="template.email.subject"
                                        sourceBodyHTML="template.email.html_body"
                                        sourceBodyPlainText="template.email.plaintext_body"
                                    />
                                </AccordionDetails>
                            </Accordion>
                        )}
                        {formData.template?.selected_methods?.SMS && (
                            <Accordion
                                sx={{width: "100%"}}
                                expanded={expandedSMS}
                                onChange={() => setExpandedSMS(!expandedSMS)}
                            >
                                <AccordionSummary
                                    expandIcon={<ExpandMoreIcon id="template-sms-id" />}
                                >
                                    <ElectionHeaderStyles.AccordionTitle>
                                        {t("template.form.smsMessage")}
                                    </ElectionHeaderStyles.AccordionTitle>
                                </AccordionSummary>
                                <AccordionDetails>
                                    <FormStyles.TextInput
                                        minRows={4}
                                        multiline={true}
                                        source="template.sms.message"
                                        label={t("template.form.smsMessage")}
                                    />
                                </AccordionDetails>
                            </Accordion>
                        )}
                        {formData.template?.selected_methods?.DOCUMENT && (
                            <Accordion
                                sx={{width: "100%"}}
                                expanded={expandedDocument}
                                onChange={() => setExpandedDocument(!expandedDocument)}
                            >
                                <AccordionSummary
                                    expandIcon={<ExpandMoreIcon id="template-document-id" />}
                                >
                                    <ElectionHeaderStyles.AccordionTitle>
                                        {t("template.form.document")}
                                    </ElectionHeaderStyles.AccordionTitle>
                                </AccordionSummary>
                                <AccordionDetails>
                                    <EmailEditEditor sourceBodyPlainText="template.document" />
                                </AccordionDetails>
                            </Accordion>
                        )}
                    </>
                )}
            </FormDataConsumer>
        </FormControl>
    )
}
