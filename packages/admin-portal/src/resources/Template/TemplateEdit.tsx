// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"

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

import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {
    EditBase,
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
import {FormStyles} from "@/components/styles/FormStyles"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {useMutation} from "@apollo/client"

import {ICommunicationType, ICommunicationMethod} from "@/types/communications"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"
import EmailEditEditor from "@/components/EmailEditEditor"
import {Sequent_Backend_Template} from "@/gql/graphql"
import {UPDATE_template} from "@/queries/UpdateCommunicationTemplate"

type TTemplateEdit = {
    id?: Identifier | undefined
    close?: () => void
}

export const TemplateEdit: React.FC<TTemplateEdit> = (props) => {
    const {id, close} = props

    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const refresh = useRefresh()
    const notify = useNotify()

    const [UpdateCommunicationTemplate] = useMutation(UPDATE_template)

    const communicationTypeChoices = () => {
        return (Object.values(ICommunicationType) as ICommunicationType[]).map((value) => ({
            id: value,
            name: t(`communicationTemplate.type.${value.toLowerCase()}`),
        }))
    }
    const [selectedCommunicationType, setSelectedCommunicationType] = useState<{
        name: string
        value: ICommunicationType
    }>()

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
        console.log("Submit Template", data)

        const {data: updated, errors} = await UpdateCommunicationTemplate({
            variables: {
                id: id,
                tenantId: tenantId,
                set: {...data},
            },
        })

        if (updated) {
            notify("communicationTemplate.update.success", {type: "success"})
        }

        if (errors) {
            notify("communicationTemplate.update.error", {type: "error"})
        }

        close?.()
    }

    const onSuccess = async (res: any) => {
        console.log("onSuccess :>> ", res)

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

    const parseValues = (incoming: RaRecord<Identifier> | Omit<RaRecord<Identifier>, "id">) => {
        const temp = {...(incoming as Sequent_Backend_Template)}
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
        <EditBase
            id={id}
            resource="sequent_backend_template"
            mutationMode="pessimistic"
            mutationOptions={{onSuccess, onError}}
            redirect={false}
        >
            <PageHeaderStyles.Wrapper>
                <RecordContext.Consumer>
                    {(incoming) => {
                        const parsedValue: RaRecord<Identifier> | Omit<RaRecord<Identifier>, "id"> =
                            parseValues(incoming)
                        console.log("parsedValue :>> ", parsedValue)

                        return (
                            <SimpleForm
                                record={parsedValue}
                                onSubmit={onSubmit}
                                toolbar={<SaveButton />}
                            >
                                <FormControl fullWidth>
                                    <ElectionHeaderStyles.Wrapper>
                                        <PageHeaderStyles.Title>
                                            {t("communicationTemplate.edit.title")}
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
                                                label={t("communicationTemplate.form.alias")}
                                            />

                                            <FormStyles.TextInput
                                                source="template.name"
                                                validate={required()}
                                                label={t("communicationTemplate.form.name")}
                                            />
                                            <SelectInput
                                                source="type"
                                                label={"Template type"}
                                                validate={required()}
                                                choices={communicationTypeChoices()}
                                                onChange={(e) => {
                                                    const selectedType = e.target
                                                        .value as ICommunicationType
                                                    setSelectedCommunicationType({
                                                        name: t(
                                                            `communicationTemplate.type.${selectedType.toLowerCase()}`
                                                        ),
                                                        value: selectedType,
                                                    })
                                                }}
                                            />
                                        </AccordionDetails>
                                    </Accordion>

                                    <FormLabel component="legend">Choose Methods</FormLabel>
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
                                                    {t("communicationTemplate.method.email")}
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
                                                    {t("communicationTemplate.form.smsMessage")}
                                                </ElectionHeaderStyles.AccordionTitle>
                                            </AccordionSummary>
                                            <AccordionDetails>
                                                <FormStyles.TextInput
                                                    minRows={4}
                                                    multiline={true}
                                                    source="template.sms.message"
                                                    label={t(
                                                        "communicationTemplate.form.smsMessage"
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
                                                    {t("communicationTemplate.form.document")}
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
        </EditBase>
    )
}
