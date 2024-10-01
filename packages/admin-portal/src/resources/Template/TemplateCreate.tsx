// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useState} from "react"

import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    Checkbox,
    FormControl,
    FormControlLabel,
    FormGroup,
    FormLabel,
} from "@mui/material"

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
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {ITemplateType, ICommunicationMethod, ISendCommunicationBody} from "@/types/templates"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {INSERT_TEMPLATE} from "@/queries/InsertTemplate"
import {Sequent_Backend_Template} from "@/gql/graphql"
import EmailEditEditor from "@/components/EmailEditEditor"
import {SettingsContext} from "@/providers/SettingsContextProvider"

type TTemplateCreate = {
    close?: () => void
}

export const TemplateCreate: React.FC<TTemplateCreate> = ({close}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const notify = useNotify()
    const [createTemplate] = useMutation(INSERT_TEMPLATE)
    const {globalSettings} = useContext(SettingsContext)

    const [selectedCommunicationType, setSelectedCommunicationType] = useState<{
        name: string
        value: ITemplateType
    }>()

    function selectCommunicationType(event: any) {
        const choice = event.target
        setSelectedCommunicationType(choice)
    }

    const communicationTypeChoices = () => {
        return (Object.values(ITemplateType) as ITemplateType[]).map((value) => ({
            id: value,
            name: t(`template.type.${value.toLowerCase()}`),
        }))
    }

    const communicationMethodChoices = () => {
        let res = (Object.values(ICommunicationMethod) as ICommunicationMethod[]).map((value) => ({
            id: value,
            name: t(`template.method.${value.toLowerCase()}`),
        }))

        if (
            selectedCommunicationType?.value &&
            ![ITemplateType.BALLOT_RECEIPT, ITemplateType.TALLY_REPORT].includes(
                selectedCommunicationType.value
            )
        ) {
            res = res.filter((cm) => cm.id !== ICommunicationMethod.DOCUMENT)
        }
        if (ITemplateType.TALLY_REPORT === selectedCommunicationType?.value) {
            res = res.filter((cm) => cm.id === ICommunicationMethod.DOCUMENT)
        }

        return res
    }

    const onSubmit: SubmitHandler<FieldValues> = async (data) => {
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

    const parseValues = (incoming: RaRecord<Identifier> | Omit<RaRecord<Identifier>, "id">) => {
        const temp = {...(incoming as Sequent_Backend_Template)}

        if (!incoming?.template) {
            temp.type = ITemplateType.CREDENTIALS
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

    const [expandedGeneral, setExpandedGeneral] = useState<boolean>(true)
    const [expandedEmail, setExpandedEmail] = useState<boolean>(false)
    const [expandedSMS, setExpandedSMS] = useState<boolean>(false)
    const [expandedDocument, setExpandedDocument] = useState<boolean>(false)
    const [methods, setMethods] = React.useState({
        [ICommunicationMethod.EMAIL]: false,
        [ICommunicationMethod.SMS]: false,
        [ICommunicationMethod.DOCUMENT]: false,
    })

    const handleChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        setMethods({
            ...methods,
            [event.target.name]: event.target.checked,
        })
    }

    return (
        <CreateBase resource="sequent_backend_template" redirect={false}>
            <PageHeaderStyles.Wrapper>
                <RecordContext.Consumer>
                    {(incoming) => {
                        const parsedValue: RaRecord<Identifier> | Omit<RaRecord<Identifier>, "id"> =
                            parseValues(incoming)

                        return (
                            <SimpleForm record={parsedValue} onSubmit={onSubmit}>
                                <FormControl fullWidth>
                                    <ElectionHeaderStyles.Wrapper>
                                        <PageHeaderStyles.Title>
                                            {t("template.create.title")}
                                        </PageHeaderStyles.Title>
                                    </ElectionHeaderStyles.Wrapper>
                                    <Accordion
                                        sx={{width: "100%"}}
                                        expanded={expandedGeneral}
                                        onChange={() => setExpandedGeneral(!expandedGeneral)}
                                    >
                                        <AccordionSummary
                                            expandIcon={
                                                <ExpandMoreIcon id="election-event-data-general" />
                                            }
                                        >
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
                                                onChange={selectCommunicationType}
                                                choices={communicationTypeChoices()}
                                            />
                                        </AccordionDetails>
                                    </Accordion>
                                    <FormLabel component="legend">
                                        {t(`template.chooseMethods`)}
                                    </FormLabel>
                                    <FormGroup
                                        sx={{
                                            display: "flex",
                                            flexDirection: "row",
                                            gap: "16px",
                                        }}
                                    >
                                        {communicationMethodChoices().map((method) => (
                                            <FormControlLabel
                                                key={method.id}
                                                control={
                                                    <Checkbox
                                                        checked={
                                                            methods[
                                                                method.id as ICommunicationMethod
                                                            ]
                                                        }
                                                        onChange={handleChange}
                                                        name={method.id}
                                                    />
                                                }
                                                label={method.name}
                                            />
                                        ))}
                                    </FormGroup>
                                    {methods[ICommunicationMethod.EMAIL] && (
                                        <Accordion
                                            sx={{width: "100%"}}
                                            expanded={expandedEmail}
                                            onChange={() => setExpandedEmail(!expandedEmail)}
                                        >
                                            <AccordionSummary
                                                expandIcon={
                                                    <ExpandMoreIcon id="communication-template-email-id" />
                                                }
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
                                    {methods[ICommunicationMethod.SMS] && (
                                        <Accordion
                                            sx={{width: "100%"}}
                                            expanded={expandedSMS}
                                            onChange={() => setExpandedSMS(!expandedSMS)}
                                        >
                                            <AccordionSummary
                                                expandIcon={
                                                    <ExpandMoreIcon id="communication-template-sms-id" />
                                                }
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
                                                    label={t(
                                                        "template.form.smsMessage"
                                                    )}
                                                />
                                            </AccordionDetails>
                                        </Accordion>
                                    )}
                                    {methods[ICommunicationMethod.DOCUMENT] && (
                                        <Accordion
                                            sx={{width: "100%"}}
                                            expanded={expandedDocument}
                                            onChange={() => setExpandedDocument(!expandedDocument)}
                                        >
                                            <AccordionSummary
                                                expandIcon={
                                                    <ExpandMoreIcon id="communication-template-document-id" />
                                                }
                                            >
                                                <ElectionHeaderStyles.AccordionTitle>
                                                    {t("template.form.document")}
                                                </ElectionHeaderStyles.AccordionTitle>
                                            </AccordionSummary>
                                            <AccordionDetails>
                                                <EmailEditEditor sourceBodyHTML="template.document" />
                                            </AccordionDetails>
                                        </Accordion>
                                    )}
                                </FormControl>
                            </SimpleForm>
                        )
                    }}
                </RecordContext.Consumer>
            </PageHeaderStyles.Wrapper>
        </CreateBase>
    )
}
