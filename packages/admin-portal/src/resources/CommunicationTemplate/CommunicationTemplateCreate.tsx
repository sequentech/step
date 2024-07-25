// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useState} from "react"

import styled from "@emotion/styled"

import {AccordionDetails, AccordionSummary, FormControl} from "@mui/material"

import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {
    CreateBase,
    Identifier,
    RaRecord,
    RecordContext,
    SelectInput,
    SimpleForm,
    required,
    useNotify,
} from "react-admin"

import {FieldValues, SubmitHandler} from "react-hook-form"
import {FormStyles} from "@/components/styles/FormStyles"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {useMutation} from "@apollo/client"

import {
    ICommunicationType,
    ICommunicationMethod,
    ISendCommunicationBody,
} from "@/types/communications"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {INSERT_COMMUNICATION_TEMPLATE} from "@/queries/InsertCommunicationTemplate"
import {useWatch} from "react-hook-form"
import {Sequent_Backend_Communication_Template} from "@/gql/graphql"
import EmailEditEditor from "@/components/EmailEditEditor"
import {SettingsContext} from "@/providers/SettingsContextProvider"

const CommunicationTemplateCreateStyle = {
    Box: styled.div`
        display: flex;
        flex-direction: column;
        border: 1px solid #f2f2f2;
        padding: 8px;
        border-radius: 4px;
        margin-bottom: 16px;
    `,
    BoxTitle: styled.span`
        font-family: Roboto;
        font-size: 24px;
        font-weight: 700;
        line-height: 32px;
        letter-spacing: 0px;
        text-align: left;
        padding: 0 16px;
    `,
    ContainerLanguage: styled.div``,
    RichText: styled.div`
        border: 1px solid #f2f2f2;
        border-radius: 4px;
        width: 100%;
        padding: 0;
    `,
}

type TCommunicationTemplateCreate = {
    close?: () => void
}

const CommunicationTemplateTitleContainer: React.FC<any> = ({children, title}) => {
    return (
        <CommunicationTemplateCreateStyle.Box>
            <CommunicationTemplateCreateStyle.BoxTitle>
                {title}
            </CommunicationTemplateCreateStyle.BoxTitle>

            {children}
        </CommunicationTemplateCreateStyle.Box>
    )
}

export const ContentInput: React.FC = () => {
    const {t} = useTranslation()
    const communicationMethod = useWatch({name: "communication_method"})

    switch (communicationMethod) {
        case ICommunicationMethod.EMAIL:
            return (
                <EmailEditEditor
                    key={`editor-${communicationMethod}`}
                    sourceSubject="template.email.subject"
                    sourceBodyHTML="template.email.html_body"
                    sourceBodyPlainText="template.email.plaintext_body"
                />
            )

        case ICommunicationMethod.SMS:
            return (
                <FormStyles.TextInput
                    minRows={4}
                    multiline={true}
                    source="template.sms.message"
                    label={t("communicationTemplate.form.smsMessage")}
                />
            )

        case ICommunicationMethod.DOCUMENT:
            return (
                <EmailEditEditor
                    key={`editor-${communicationMethod}`}
                    sourceBodyHTML="template.document"
                    sourceBodyPlainText="template.document"
                />
            )
    }

    return <></>
}

export const CommunicationTemplateCreate: React.FC<TCommunicationTemplateCreate> = ({close}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const notify = useNotify()
    const [createCommunicationTemplate] = useMutation(INSERT_COMMUNICATION_TEMPLATE)
    const {globalSettings} = useContext(SettingsContext)

    const [selectedCommunicationType, setSelectedCommunicationType] = useState<{
        name: string
        value: ICommunicationType
    }>()

    function selectCommunicationType(event: any) {
        const choice = event.target
        setSelectedCommunicationType(choice)
    }

    const communicationTypeChoices = () => {
        return (Object.values(ICommunicationType) as ICommunicationType[]).map((value) => ({
            id: value,
            name: t(`communicationTemplate.type.${value.toLowerCase()}`),
        }))
    }

    const communicationMethodChoices = () => {
        let res = (Object.values(ICommunicationMethod) as ICommunicationMethod[]).map((value) => ({
            id: value,
            name: t(`communicationTemplate.method.${value.toLowerCase()}`),
        }))

        if (
            selectedCommunicationType?.value &&
            ![ICommunicationType.BALLOT_RECEIPT, ICommunicationType.TALLY_REPORT].includes(
                selectedCommunicationType.value
            )
        ) {
            res = res.filter((cm) => cm.id !== ICommunicationMethod.DOCUMENT)
        }
        if (ICommunicationType.TALLY_REPORT === selectedCommunicationType?.value) {
            res = res.filter((cm) => cm.id === ICommunicationMethod.DOCUMENT)
        }

        return res
    }

    const onSubmit: SubmitHandler<FieldValues> = async (data) => {
        const {data: created, errors} = await createCommunicationTemplate({
            variables: {
                object: {
                    tenant_id: tenantId,
                    communication_type: data.communication_type,
                    communication_method: data.communication_method,
                    template: {
                        ...data.template,
                    },
                },
            },
        })

        if (created) {
            notify(t("communicationTemplate.create.success"), {type: "success"})
        }

        if (errors) {
            notify(t("communicationTemplate.create.error"), {type: "error"})
        }

        close?.()
    }

    const parseValues = (incoming: RaRecord<Identifier> | Omit<RaRecord<Identifier>, "id">) => {
        const temp = {...(incoming as Sequent_Backend_Communication_Template)}

        if (!incoming?.template) {
            temp.communication_type = ICommunicationType.CREDENTIALS
            temp.communication_method = ICommunicationMethod.EMAIL
            let template: ISendCommunicationBody = {
                audience_selection: undefined,
                audience_voter_ids: [],
                schedule_date: undefined,
                schedule_now: undefined,
                email: {
                    subject: globalSettings.DEFAULT_EMAIL_SUBJECT["en"] ?? "",
                    plaintext_body: globalSettings.DEFAULT_EMAIL_PLAINTEXT_BODY["en"] ?? "",
                    html_body: globalSettings.DEFAULT_EMAIL_HTML_BODY["en"] ?? "",
                },
                sms: {
                    message: globalSettings.DEFAULT_SMS_MESSAGE["en"] ?? "",
                },
                document: globalSettings.DEFAULT_DOCUMENT["en"] ?? "",
            }
            temp.template = template
        }

        return temp
    }

    return (
        <CreateBase resource="sequent_backend_communication_template" redirect={false}>
            <PageHeaderStyles.Wrapper>
                <RecordContext.Consumer>
                    {(incoming) => {
                        const parsedValue: RaRecord<Identifier> | Omit<RaRecord<Identifier>, "id"> =
                            parseValues(incoming)

                        return (
                            <SimpleForm record={parsedValue} onSubmit={onSubmit}>
                                <PageHeaderStyles.Title>
                                    {t("communicationTemplate.edit.title")}
                                </PageHeaderStyles.Title>

                                <FormStyles.TextInput
                                    source="template.alias"
                                    validate={required()}
                                    // onChange={handleInputChange}
                                    label={t("communicationTemplate.form.alias")}
                                />

                                <FormStyles.TextInput
                                    source="template.name"
                                    validate={required()}
                                    // onChange={handleInputChange}
                                    label={t("communicationTemplate.form.name")}
                                />

                                <FormControl fullWidth>
                                    <CommunicationTemplateTitleContainer
                                        title={t("communicationTemplate.form.communicationType")}
                                    >
                                        <SelectInput
                                            source="communication_type"
                                            validate={required()}
                                            onChange={selectCommunicationType}
                                            choices={communicationTypeChoices()}
                                        />
                                    </CommunicationTemplateTitleContainer>

                                    <FormStyles.AccordionExpanded expanded={true} disableGutters>
                                        <AccordionSummary
                                            expandIcon={
                                                <ExpandMoreIcon id="communication-template-method-id" />
                                            }
                                        >
                                            <ElectionHeaderStyles.Wrapper>
                                                <ElectionHeaderStyles.Title>
                                                    {t(
                                                        "communicationTemplate.form.communicationMethod"
                                                    )}
                                                </ElectionHeaderStyles.Title>
                                            </ElectionHeaderStyles.Wrapper>
                                        </AccordionSummary>
                                        <AccordionDetails>
                                            <SelectInput
                                                source="communication_method"
                                                validate={required()}
                                                choices={communicationMethodChoices()}
                                            />
                                            <ContentInput />
                                        </AccordionDetails>
                                    </FormStyles.AccordionExpanded>
                                </FormControl>
                            </SimpleForm>
                        )
                    }}
                </RecordContext.Consumer>
            </PageHeaderStyles.Wrapper>
        </CreateBase>
    )
}
