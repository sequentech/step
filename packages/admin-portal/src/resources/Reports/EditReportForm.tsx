// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import SelectElection from "@/components/election/SelectElection"
import {EReportElectionPolicy, EReportType, ReportActions, reportTypeConfig} from "@/types/reports"
import {Typography, Autocomplete, Chip, TextField, Box} from "@mui/material"
import React, {useEffect, useMemo, useState} from "react"
import {
    BooleanInput,
    Create,
    Identifier,
    SaveButton,
    SelectInput,
    SimpleForm,
    Toolbar,
    useGetOne,
    useNotify,
    useInput,
    InputProps,
} from "react-admin"
import SelectTemplate from "../Template/SelectTemplate"
import {useTranslation} from "react-i18next"
import {EncryptReportMutation, Sequent_Backend_Report} from "@/gql/graphql"
import {useMutation} from "@apollo/client"
import {CREATE_REPORT} from "@/queries/CreateReport"
import {UPDATE_REPORT} from "@/queries/UpdateReport"
import {ETemplateType} from "@/types/templates"
import {useFormContext} from "react-hook-form"
import {Cron} from "react-js-cron"
import "react-js-cron/dist/styles.css"
import {ENCRYPT_REPORT} from "@/queries/EncryptReport"
import {IPermissions} from "@/types/keycloak"
import {Dialog} from "@sequentech/ui-essentials"

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
    setEnabled?: (v: boolean) => void
}

export enum EReportEncryption {
    UNENCRYPTED = "unencrypted",
    CONFIGURED_PASSWORD = "configured_password",
}

export const EditReportForm: React.FC<CreateReportProps> = ({
    close,
    tenantId,
    electionEventId,
    isEditReport,
    reportId,
}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const [reportEncryptionPolicy, setReportEncryptionPolicy] = useState<
        EReportEncryption | undefined
    >(EReportEncryption.UNENCRYPTED)
    const [reportIsEncrypted, setReportIsEncrypted] = useState<boolean>(false)
    const [createReport] = useMutation(CREATE_REPORT)
    const [updateReport] = useMutation(UPDATE_REPORT)
    const [encryptReport] = useMutation<EncryptReportMutation>(ENCRYPT_REPORT, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.REPORT_WRITE,
            },
        },
    })
    const [handlePasswordDialogOpen, setHandlePasswordDialogOpen] = useState<boolean>(false)
    const [filePassword, setFilePassword] = useState({password: "", confirmPassword: ""})

    const [isCronActive, setIsCronActive] = useState<boolean>(false)
    const [cronValue, setCronValue] = useState<string>("00 8 * * 1,2,3,4,5")
    const [enabled, setEnabled] = useState<boolean>(false)

    const reportEncryptionPolicyChoices = Object.keys(EReportEncryption).map((key) => ({
        id: EReportEncryption[key as keyof typeof EReportEncryption],
        name: t(`reportsScreen.reportEncryptionPolicy.${key}`),
    }))

    const {
        data: report,
        isLoading,
        error,
    } = useGetOne<Sequent_Backend_Report>(
        "sequent_backend_report",
        {id: reportId},
        {enabled: isEditReport}
    )

    const handleSubmit = async (values: any) => {
        const formData: Partial<Sequent_Backend_Report> = {
            ...values,
            encryption_policy: values.encryption_policy,
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
                await encryptReport({
                    variables: {
                        electionEventId: electionEventId,
                        reportId: reportId,
                        password: filePassword.password,
                    },
                    onCompleted: async (data) => {
                        if (data.encrypt_report?.error_msg) {
                            notify(data.encrypt_report.error_msg, {type: "error"})
                        } else {
                            setReportIsEncrypted(true)
                            setHandlePasswordDialogOpen(false)
                            notify(t("reportsScreen.messages.encryptSuccess"), {type: "success"})
                            await updateReport({
                                variables: {
                                    id: reportId,
                                    set: formData,
                                },
                            })
                            notify(t(`reportsScreen.messages.updateSuccess`), {type: "success"})
                        }
                    },
                    onError: (error) => {
                        notify(t("reportsScreen.messages.encryptError"), {type: "error"})
                    },
                })
            } else {
                const {data: reportData} = await createReport({
                    variables: {
                        object: formData,
                    },
                })
                notify(t(`reportsScreen.messages.createSuccess`), {type: "success"})
                if (reportData) {
                    await encryptReport({
                        variables: {
                            reportId: reportData.insert_sequent_backend_report.returning[0].id,
                            electionEventId: electionEventId,
                            password: filePassword.password,
                        },
                        onCompleted: (data) => {
                            if (data.encrypt_report?.error_msg) {
                                notify(data.encrypt_report.error_msg, {type: "error"})
                            } else {
                                setReportIsEncrypted(true)
                                setHandlePasswordDialogOpen(false)
                                notify(t("reportsScreen.messages.encryptSuccess"), {
                                    type: "success",
                                })
                            }
                        },
                        onError: (error) => {
                            notify(t("reportsScreen.messages.encryptError"), {type: "error"})
                        },
                    })
                }
            }

            if (close) {
                close()
            }
        } catch (error) {
            notify(t(`reportsScreen.messages.submitError`), {type: "error"})
        }
    }
    const shouldOpenPasswordDialog = useMemo(() => {
        return reportEncryptionPolicy === EReportEncryption.CONFIGURED_PASSWORD
    }, [reportEncryptionPolicy])

    useEffect(() => {
        if (shouldOpenPasswordDialog) {
            setHandlePasswordDialogOpen(true)
        }
    }, [shouldOpenPasswordDialog])

    const onEncryptReport = async () => {
        await encryptReport({
            variables: {
                electionEventId: electionEventId,
                reportId: report?.id || "",
                password: filePassword.password,
            },
            onCompleted: (data) => {
                if (data.encrypt_report?.error_msg) {
                    notify(data.encrypt_report.error_msg, {type: "error"})
                } else {
                    setReportIsEncrypted(true)
                    setHandlePasswordDialogOpen(false)
                    notify(t("reportsScreen.messages.encryptSuccess"), {type: "success"})
                }
            },
            onError: (error) => {
                notify(t("reportsScreen.messages.encryptError"), {type: "error"})
            },
        })
    }

    return (
        <>
            <Create hasEdit={isEditReport}>
                <SimpleForm
                    record={isEditReport ? report : undefined}
                    onSubmit={handleSubmit}
                    toolbar={
                        <Toolbar>
                            <SaveButton alwaysEnable={enabled} />
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
                        setEnabled={setEnabled}
                    />
                    <SelectInput
                        source={"encryption_policy"}
                        label={t("reportsScreen.reportEncryptionPolicy.title")}
                        choices={reportEncryptionPolicyChoices}
                        disabled={
                            report?.encryption_policy === EReportEncryption.CONFIGURED_PASSWORD
                        }
                        defaultValue={EReportEncryption.UNENCRYPTED}
                        onChange={(event) => {
                            setReportEncryptionPolicy(event.target.value)
                        }}
                        value={report?.encryption_policy}
                        isRequired
                    />
                </SimpleForm>
            </Create>
            <Dialog
                variant="info"
                open={handlePasswordDialogOpen}
                handleClose={(result: boolean) => {
                    if (result) {
                        if (filePassword.password === filePassword.confirmPassword) {
                            setReportIsEncrypted(true)
                            setHandlePasswordDialogOpen(false)
                        } else {
                            notify(t("reportsScreen.messages.passwordMismatch"), {type: "error"})
                        }
                    } else {
                        setHandlePasswordDialogOpen(false)
                    }
                }}
                aria-labelledby="password-dialog-title"
                title={t("electionEventScreen.export.passwordTitle")}
                ok={"Save"}
            >
                <Box component={"form"}>
                    {"Password"}
                    <TextField
                        fullWidth
                        margin="normal"
                        type="password"
                        value={filePassword.password}
                        onChange={(e) =>
                            setFilePassword({
                                password: e.target.value,
                                confirmPassword: filePassword.confirmPassword,
                            })
                        }
                    />
                    {"Confirm password"}
                    <TextField
                        fullWidth
                        margin="normal"
                        type="password"
                        value={filePassword.confirmPassword}
                        onChange={(e) =>
                            setFilePassword({
                                password: filePassword.password,
                                confirmPassword: e.target.value,
                            })
                        }
                    />
                </Box>
            </Dialog>
        </>
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
        field, // Contains value and onChange
        fieldState, // Contains error and touched
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
                    <Chip label={option} {...getTagProps({index})} key={index} />
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

// Add 'onChange' to the props interface
interface ReportTypeInputProps extends InputProps {
    label?: string
    choices: {id: any; name: string}[]
    onChange?: (newValue: any) => void // Add this line
}

const ReportTypeInput: React.FC<ReportTypeInputProps> = (props) => {
    const {field, fieldState, isRequired, id} = useInput(props)

    const {choices, label, onChange} = props // Destructure 'onChange'

    return (
        <Autocomplete
            fullWidth={true}
            options={choices}
            getOptionLabel={(option) => option.name}
            onChange={(event: any, newValue: {id: string; name: string} | null) => {
                field.onChange(newValue ? newValue.id : null)
                if (onChange) {
                    onChange(newValue ? newValue.id : null) // Call the passed onChange prop
                }
            }}
            value={choices.find((choice) => choice.id === field.value) || null}
            renderInput={(params) => (
                <TextField
                    {...params}
                    label={label}
                    variant="outlined"
                    required={isRequired}
                    error={fieldState.invalid}
                    helperText={fieldState.error?.message}
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
    setEnabled,
}) => {
    const {t} = useTranslation()

    const [reportType, setReportType] = useState<ETemplateType | undefined>(undefined)
    const [electionId, setElectionId] = useState<string | null | undefined>(undefined)
    const [templateId, setTemplateId] = useState<string | null | undefined>(undefined)
    const [isCronActive, setIsCronActive] = useState<boolean>(false)

    const {setValue, register} = useFormContext()

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
        if (reportType) {
            setTemplateId(null)
            setValue("template_id", null)
            setValue("report_type", reportType)
        }
    }, [reportType, setValue])

    useEffect(() => {
        if (report?.election_id && electionPolicy === EReportElectionPolicy.ELECTION_NOT_ALLOWED) {
            setElectionId(null)
            setValue("election_id", null)
        }
    }, [electionPolicy])

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

    const handleReportTypeChange = (newValue: ETemplateType | null) => {
        setReportType(newValue || undefined)
        setTemplateId(null)
        setValue("template_id", null)
        setValue("report_type", newValue)
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

            <ReportTypeInput
                source="report_type"
                label={t("template.form.type")}
                choices={reportTypeChoices}
                isRequired={true}
                onChange={handleReportTypeChange}
            />

            <SelectElection
                tenantId={tenantId}
                electionEventId={electionEventId}
                label={t("reportsScreen.fields.electionId")}
                onSelectElection={(electionId) => setElectionId(electionId)}
                source="election_id"
                value={electionId}
                isRequired={electionPolicy === EReportElectionPolicy.ELECTION_REQUIRED}
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
                isRequired={isTemplateRequired}
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
                            if (newValue !== cronValue) {
                                setEnabled?.(true)
                            }
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
