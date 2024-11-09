// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import SelectElection from "@/components/election/SelectElection"
import {
    EReportElectionPolicy,
    EReportType,
    ReportActions,
    reportTypeConfig,
} from "@/types/reports"
import { Typography, Autocomplete, Chip, TextField } from "@mui/material"
import React, { useEffect, useMemo, useState } from "react"
import {
    BooleanInput,
    Create,
    Identifier,
    SaveButton,
    SelectInput,
    SimpleForm,
    TextInput,
    Toolbar,
    useGetOne,
    useNotify,
    useInput,
    InputProps,
} from "react-admin"
import SelectTemplate from "../Template/SelectTemplate"
import { useTranslation } from "react-i18next"
import { Sequent_Backend_Report } from "@/gql/graphql"
import { useMutation } from "@apollo/client"
import { CREATE_REPORT } from "@/queries/CreateReport"
import { UPDATE_REPORT } from "@/queries/UpdateReport"
import { ETemplateType } from "@/types/templates"
import { useFormContext } from "react-hook-form"
import { Cron } from "react-js-cron"
import "react-js-cron/dist/styles.css"

interface CreateReportProps {
    close?: () => void
    electionEventId: string | undefined
    tenantId: string | null
    isEditReport: boolean
    reportId?: Identifier | null | undefined
    report?: Sequent_Backend_Report | null | undefined
    doCronActive?: (isActive: boolean) => void
    cronValue?: string
    setCronValue?: (v: string) => void
}

export const EditReportForm: React.FC<CreateReportProps> = ({
    close,
    tenantId,
    electionEventId,
    isEditReport,
    reportId,
}) => {
    const { t } = useTranslation()
    const notify = useNotify()

    const [createReport] = useMutation(CREATE_REPORT)
    const [updateReport] = useMutation(UPDATE_REPORT)

    const [isCronActive, setIsCronActive] = useState<boolean>(false)
    const [cronValue, setCronValue] = useState<string>("00 8 * * 1,2,3,4,5")

    const {
        data: report,
        isLoading,
        error,
    } = useGetOne<Sequent_Backend_Report>(
        "sequent_backend_report",
        { id: reportId },
        { enabled: isEditReport }
    )

    const handleSubmit = async (values: any) => {
        const formData = {
            ...values,
            tenant_id: tenantId,
            election_event_id: electionEventId,
            cron_config: isCronActive
                ? {
                      is_active: true,
                      cron_expression: cronValue,
                      email_recipients: values.cron_config.email_recipients,
                  }
                : null,
        }

        try {
            if (isEditReport && reportId) {
                await updateReport({
                    variables: {
                        id: reportId,
                        set: formData,
                    },
                })
                notify(t(`reportsScreen.messages.updateSuccess`), { type: "success" })
            } else {
                await createReport({
                    variables: {
                        object: formData,
                    },
                })
                notify(t(`reportsScreen.messages.createSuccess`), { type: "success" })
            }

            if (close) {
                close()
            }
        } catch (error) {
            notify(t(`reportsScreen.messages.submitError`), { type: "error" })
        }
    }

    return (
        <Create hasEdit={isEditReport}>
            <SimpleForm
                record={isEditReport ? report : undefined}
                onSubmit={handleSubmit}
                toolbar={
                    <Toolbar>
                        <SaveButton />
                    </Toolbar>
                }
            >
                <FormContent
                    tenantId={tenantId}
                    electionEventId={electionEventId}
                    isEditReport={isEditReport}
                    report={report}
                    doCronActive={(value) => setIsCronActive(value)}
                    cronValue={cronValue}
                    setCronValue={setCronValue}
                />
            </SimpleForm>
        </Create>
    )
}

interface EmailRecipientsInputProps extends InputProps {
    label?: string
    placeholder?: string
}


interface EmailRecipientsInputProps extends InputProps {
    label?: string
    placeholder?: string
}

const EmailRecipientsInput: React.FC<EmailRecipientsInputProps> = (props) => {
    const {
        field,        // Contains value and onChange
        fieldState,   // Contains error and touched
        isRequired,
        id,
    } = useInput(props)

    return (
        <Autocomplete
            multiple
            freeSolo
            options={[] as string[]}
            value={field.value || []}
            onChange={(event: any, newValue: string[]) => {
                field.onChange(newValue)
            }}
            fullWidth={true}
            renderTags={(value: string[], getTagProps) =>
                value.map((option: string, index: number) => (
                    <Chip
                        label={option}
                        {...getTagProps({ index })}
                        key={index}
                    />
                ))
            }
            renderInput={(params) => (
                <TextField
                    {...params}
                    variant="outlined"
                    label={props.label}
                    placeholder={props.placeholder}
                    error={fieldState.invalid}
                    helperText={fieldState.error?.message}
                    required={isRequired}
                    id={id}
                />
            )}
        />
    )
}

const FormContent: React.FC<CreateReportProps> = ({
    tenantId,
    electionEventId,
    isEditReport,
    report,
    doCronActive,
    cronValue,
    setCronValue,
}) => {
    const { t } = useTranslation()

    const [reportType, setReportType] = useState<ETemplateType | undefined>(undefined)
    const [electionId, setElectionId] = useState<string | null | undefined>(undefined)
    const [templateId, setTemplateId] = useState<string | null | undefined>(undefined)
    const [isCronActive, setIsCronActive] = useState<boolean>(false)

    const { setValue, register } = useFormContext()

    useEffect(() => {
        register("cron_config.cron_expression")
    }, [register])

    useEffect(() => {
        console.log("report changed")
        setIsCronActive(report?.cron_config?.is_active || false)
        setCronValue?.(report?.cron_config?.cron_expression || "")
        setReportType(report?.report_type ? (report.report_type as ETemplateType) : undefined)
        setTemplateId(report?.template_id || undefined)

        setValue("template_id", report?.template_id || undefined)
        setValue(
            "report_type",
            report?.report_type ? (report.report_type as ETemplateType) : undefined
        )
        setValue("cron_config.email_recipients", report?.cron_config?.email_recipients || [])
    }, [report, setValue, setCronValue])

    useEffect(() => {
        doCronActive?.(isCronActive)
    }, [isCronActive, doCronActive])

    const handleReportTypeChange = (event: any) => {
        const newValue = event.target.value as ETemplateType
        setReportType(newValue)
        setTemplateId(null)
        setValue("template_id", null)
        setValue("report_type", newValue)
    }

    const reportTypeChoices = Object.values(EReportType).map((reportType) => ({
        id: reportType,
        name: t(`template.type.${reportType}`),
    }))

    const electionPolicy = useMemo((): EReportElectionPolicy => {
        if (!reportType) {
            return EReportElectionPolicy.ELECTION_ALLOWED
        }
        return reportTypeConfig[reportType].electionPolicy ?? EReportElectionPolicy.ELECTION_ALLOWED
    }, [reportType])

    const isTemplateRequired = useMemo((): boolean => {
        if (!reportType) {
            return false
        }
        return reportTypeConfig[reportType].templateRequired ?? false
    }, [reportType])

    const canGenerateReportScheduled = useMemo((): boolean => {
        if (!reportType) {
            return false
        }
        return reportTypeConfig[reportType].actions.includes(ReportActions.GENERATE_SCHEDULED)
    }, [reportType])

    useEffect(() => {
        // Reset the isCronActive state when the report type changes
        if (!canGenerateReportScheduled) {
            setIsCronActive(false)
        } else {
            setIsCronActive(report?.cron_config?.is_active ?? false)
        }
    }, [reportType, canGenerateReportScheduled, report])

    const handleCronToggle = (event: React.ChangeEvent<HTMLInputElement>) => {
        setIsCronActive(event.target.checked)
    }

    return (
        <>
            <Typography variant="h4">
                {isEditReport ? t("reportsScreen.edit.title") : t("reportsScreen.create.title")}
            </Typography>
            <Typography variant="body2">
                {isEditReport
                    ? t("reportsScreen.edit.subtitle")
                    : t("reportsScreen.create.subtitle")}
            </Typography>

            <SelectInput
                source="report_type"
                label={t("template.form.type")}
                choices={reportTypeChoices}
                onChange={handleReportTypeChange}
            />

            <SelectElection
                tenantId={tenantId}
                electionEventId={electionEventId}
                label={t("reportsScreen.fields.electionId")}
                onSelectElection={(electionId) => setElectionId(electionId)}
                source="election_id"
                value={electionId}
                disabled={electionPolicy === EReportElectionPolicy.ELECTION_NOT_ALLOWED}
            />

            <SelectTemplate
                tenantId={tenantId}
                templateType={
                    reportType ? reportTypeConfig[reportType]?.associatedTemplateType : undefined
                }
                source={"template_id"}
                label={t("reportsScreen.fields.template")}
                onSelectTemplate={(templateId) => {
                    console.log("Selected templateId:", templateId)
                    setTemplateId(templateId)
                }}
                value={templateId}
                isRequired={false}
            />

            {canGenerateReportScheduled && (
                <BooleanInput
                    source="cron_config.is_active"
                    label={t("reportsScreen.fields.repeatable")}
                    onChange={handleCronToggle}
                />
            )}

            {isCronActive && (
                <>
                    <Cron
                        value={cronValue ?? ""}
                        setValue={(newValue: string) => {
                            console.log(`new cron config: ${newValue}`)
                            setValue("cron_config.cron_expression", newValue, {
                                shouldDirty: true,
                                shouldTouch: true,
                            })
                            setCronValue?.(newValue)
                        }}
                    />
                    <EmailRecipientsInput
                        source="cron_config.email_recipients"
                        label={t("reportsScreen.fields.emailRecipients")}
                        placeholder={t("reportsScreen.fields.emailRecipientsPlaceholder")}
                        isRequired={false}
                    />
                </>
            )}
        </>
    )
}