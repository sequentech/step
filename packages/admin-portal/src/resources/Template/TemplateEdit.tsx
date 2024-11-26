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
import {FormStyles} from "@/components/styles/FormStyles"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {useMutation} from "@apollo/client"

import {ETemplateType, ITemplateMethod} from "@/types/templates"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"
import EmailEditEditor from "@/components/EmailEditEditor"
import {Sequent_Backend_Template} from "@/gql/graphql"
import {UPDATE_TEMPLATE} from "@/queries/UpdateTemplate"

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
    const [expandedGeneral, setExpandedGeneral] = useState<boolean>(true)
    const [expandedEmail, setExpandedEmail] = useState<boolean>(false)
    const [expandedSMS, setExpandedSMS] = useState<boolean>(false)
    const [expandedDocument, setExpandedDocument] = useState<boolean>(false)
    const [methods, setMethods] = React.useState({
        [ITemplateMethod.EMAIL]: false,
        [ITemplateMethod.SMS]: false,
        [ITemplateMethod.DOCUMENT]: false,
    })

    const handleChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        setMethods({
            ...methods,
            [event.target.name]: event.target.checked,
        })
    }
    const [UpdateTemplate] = useMutation(UPDATE_TEMPLATE)

    const templateTypeChoices = () => {
        return (Object.values(ETemplateType) as ETemplateType[]).map((value) => ({
            id: value,
            name: t(`template.type.${value}`),
        }))
    }
    const [selectedTemplateType, setSelectedTemplateType] = useState<{
        name: string
        value: ETemplateType
    }>()

    const onSubmit: SubmitHandler<FieldValues> = async (data) => {
        const {data: updated, errors} = await UpdateTemplate({
            variables: {
                id: id,
                tenantId: tenantId,
                set: {...data},
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

    //TODO: Use the same logic as the template to create and fetch the Hbs for the relevant document data
    const parseValues = (incoming: RaRecord<Identifier> | Omit<RaRecord<Identifier>, "id">) => {
        const temp = {...(incoming as Sequent_Backend_Template)}
        return temp
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
                                            {t("template.edit.title")}
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
                                                choices={templateTypeChoices()}
                                                onChange={(e) => {
                                                    const selectedType = e.target
                                                        .value as ETemplateType
                                                    setSelectedTemplateType({
                                                        name: t(
                                                            `template.type.${selectedType.toLowerCase()}`
                                                        ),
                                                        value: selectedType,
                                                    })
                                                }}
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
                                                        onChange={() =>
                                                            setExpandedEmail(!expandedEmail)
                                                        }
                                                    >
                                                        <AccordionSummary
                                                            expandIcon={
                                                                <ExpandMoreIcon id="template-email-id" />
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
                                                {formData.template?.selected_methods?.SMS && (
                                                    <Accordion
                                                        sx={{width: "100%"}}
                                                        expanded={expandedSMS}
                                                        onChange={() =>
                                                            setExpandedSMS(!expandedSMS)
                                                        }
                                                    >
                                                        <AccordionSummary
                                                            expandIcon={
                                                                <ExpandMoreIcon id="template-sms-id" />
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
                                                {formData.template?.selected_methods?.DOCUMENT && (
                                                    <Accordion
                                                        sx={{width: "100%"}}
                                                        expanded={expandedDocument}
                                                        onChange={() =>
                                                            setExpandedDocument(!expandedDocument)
                                                        }
                                                    >
                                                        <AccordionSummary
                                                            expandIcon={
                                                                <ExpandMoreIcon id="template-document-id" />
                                                            }
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
                            </SimpleForm>
                        )
                    }}
                </RecordContext.Consumer>
            </PageHeaderStyles.Wrapper>
        </EditBase>
    )
}
