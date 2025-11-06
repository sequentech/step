// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState, useMemo} from "react"
import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    FormControl,
    FormLabel,
    FormGroup,
} from "@mui/material"
import {SelectInput, required, BooleanInput, FormDataConsumer, useEditContext} from "react-admin"
import {Sequent_Backend_Template} from "@/gql/graphql"
import {FormStyles} from "@/components/styles/FormStyles"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {useMutation} from "@apollo/client"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {
    ETemplateType,
    ITemplateMethod,
    IExtraConfig,
    IEmail,
    ISmsConfig,
    IPdfOptions,
    IReportOptions,
} from "@/types/templates"
import {useTranslation} from "react-i18next"
import EmailEditEditor from "@/components/EmailEditEditor"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {GET_USER_TEMPLATE} from "@/queries/GetUserTemplate"
import {IPermissions} from "@/types/keycloak"
import {useFormContext} from "react-hook-form"
import {JsonEditor, UpdateFunction} from "json-edit-react"
import {report} from "process"

type TTemplateFormContent = {
    isTemplateEdit: boolean
    onFormChanged?: () => void
}

export const TemplateFormContent: React.FC<TTemplateFormContent> = ({
    isTemplateEdit = false,
    onFormChanged,
}) => {
    const {t} = useTranslation()
    const {setValue} = useFormContext()
    const {globalSettings} = useContext(SettingsContext)
    const [expandedGeneral, setExpandedGeneral] = useState(true)
    const [expandedEmail, setExpandedEmail] = useState(false)
    const [expandedSMS, setExpandedSMS] = useState(false)
    const [expandedReportOptions, setExpandedReportOptions] = useState(false)
    const [expandedDocument, setExpandedDocument] = useState(false)
    const [expandedPdfOptions, setExpandedPdfOptions] = useState(false)
    const [templateHbsData, setTemplateHbsData] = useState<string | undefined>(undefined)
    const [templateExtraConfig, setTemplateExtraConfig] = useState<IExtraConfig | undefined>(
        undefined
    )
    const {record} = useEditContext<Sequent_Backend_Template>()
    const recordMemo = useMemo(() => {
        console.log("record: ", record) // Data stored on the table
        return record ?? null
    }, [record])

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
        return (Object.values(ETemplateType) as ETemplateType[]).sort().map((value) => {
            return {
                id: value,
                name: t(`template.type.${value}`),
            }
        })
    }

    useEffect(() => {
        console.log("Fetch data EFFECT.")
        const fetchDefaultTemplateData = async () => {
            try {
                const currType = selectedTemplateType?.value as ETemplateType
                const {data: templateData, errors} = await GetUserTemplate({
                    variables: {
                        template_type: currType.toLowerCase() as string,
                    },
                })
                setTemplateHbsData(templateData?.get_user_template.template_hbs)

                const extraConfig = JSON.parse(
                    templateData?.get_user_template.extra_config
                ) as IExtraConfig
                setTemplateExtraConfig(extraConfig)
            } catch (error) {
                console.error("Error fetching template data:", error)
            }
        }

        if (!recordMemo && isTemplateEdit) return

        if (
            isTemplateEdit &&
            (!selectedTemplateType || selectedTemplateType?.value == recordMemo?.type)
            // To behave if user clicks on Edit, switches to a different type and later switch back to the original one.
        ) {
            console.log("Use Database template")
            setTemplateHbsData(recordMemo?.template?.document)
            const extraConfig = {
                communication_templates: {
                    sms_config: recordMemo?.template?.sms,
                    email_config: recordMemo?.template?.email,
                },
                pdf_options: recordMemo?.template?.pdf_options,
                report_options: recordMemo?.template?.report_options,
            }
            setTemplateExtraConfig(extraConfig)
        } else {
            console.log("Use default user template")
            fetchDefaultTemplateData()
        }
    }, [selectedTemplateType, recordMemo])

    useEffect(() => {
        setValue(
            "template.document",
            templateHbsData || globalSettings.DEFAULT_DOCUMENT["en"] || ""
        )
    }, [templateHbsData])

    useEffect(() => {
        setValue("template.pdf_options", (templateExtraConfig?.pdf_options as IPdfOptions) || "")
        setValue(
            "template.report_options",
            (templateExtraConfig?.report_options as IReportOptions) || {}
        )
        setValue(
            "template.email",
            (templateExtraConfig?.communication_templates?.email_config as IEmail) || ""
        )
        setValue(
            "template.sms",
            (templateExtraConfig?.communication_templates?.sms_config as ISmsConfig) || ""
        )
    }, [templateExtraConfig])

    type UpdateFunctionProps = Parameters<UpdateFunction>[0]

    const updatePdfOptions = ({newData}: UpdateFunctionProps) => {
        console.log("Updating PDF options...")
        setValue("template.pdf_options", (newData as IPdfOptions) || "")
        onFormChanged?.()
    }

    const updateReportOptions = ({newData}: UpdateFunctionProps) => {
        console.log("Updating report options...")
        setValue("template.report_options", (newData as IReportOptions) || "")
        onFormChanged?.()
    }

    return (
        <FormControl fullWidth>
            <ElectionHeaderStyles.Wrapper>
                <PageHeaderStyles.Title>
                    {isTemplateEdit ? t("template.edit.title") : t("template.create.title")}
                </PageHeaderStyles.Title>
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
                            <>
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
                                <Accordion
                                    sx={{width: "100%"}}
                                    expanded={expandedPdfOptions}
                                    onChange={() => setExpandedPdfOptions(!expandedPdfOptions)}
                                >
                                    <AccordionSummary
                                        expandIcon={<ExpandMoreIcon id="template-pdf-options-id" />}
                                    >
                                        <ElectionHeaderStyles.AccordionTitle>
                                            {t("template.form.pdfOptions")}
                                        </ElectionHeaderStyles.AccordionTitle>
                                    </AccordionSummary>
                                    <AccordionDetails>
                                        <JsonEditor
                                            data={templateExtraConfig?.pdf_options as IPdfOptions}
                                            onUpdate={(data) =>
                                                updatePdfOptions(data as UpdateFunctionProps)
                                            }
                                        />
                                    </AccordionDetails>
                                </Accordion>
                                <Accordion
                                    sx={{width: "100%"}}
                                    expanded={expandedReportOptions}
                                    onChange={() =>
                                        setExpandedReportOptions(!expandedReportOptions)
                                    }
                                >
                                    <AccordionSummary
                                        expandIcon={
                                            <ExpandMoreIcon id="template-report-options-id" />
                                        }
                                    >
                                        <ElectionHeaderStyles.AccordionTitle>
                                            {t("template.form.reportOptions")}
                                        </ElectionHeaderStyles.AccordionTitle>
                                    </AccordionSummary>
                                    <AccordionDetails>
                                        <JsonEditor
                                            data={
                                                templateExtraConfig?.report_options as unknown as IReportOptions
                                            }
                                            onUpdate={(data) =>
                                                updateReportOptions(data as UpdateFunctionProps)
                                            }
                                        />
                                    </AccordionDetails>
                                </Accordion>
                            </>
                        )}
                    </>
                )}
            </FormDataConsumer>
        </FormControl>
    )
}
