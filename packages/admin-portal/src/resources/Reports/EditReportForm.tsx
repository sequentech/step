// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import SelectElection from "@/components/election/SelectElection"
import {EReportElectionPolicy, EReportType, ReportActions, reportTypeConfig} from "@/types/reports"
import {Box, TextField, Typography} from "@mui/material"
import React, {useEffect, useMemo, useState} from "react"
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
} from "react-admin"
import SelectTemplate from "../Template/SelectTemplate"
import {useTranslation} from "react-i18next"
import {EncryptReportMutation, Sequent_Backend_Report} from "@/gql/graphql"
import {useMutation} from "@apollo/client"
import {CREATE_REPORT} from "@/queries/CreateReport"
import {UPDATE_REPORT} from "@/queries/UpdateReport"
import {Dialog} from "@sequentech/ui-essentials"
import {ENCRYPT_REPORT} from "@/queries/EncryptReport"
import {IPermissions} from "@/types/keycloak"
import {ETemplateType} from "@/types/templates"

interface CronConfig {
    isActive?: boolean
    cronExpression?: string
    emailRecipient?: string
}

interface CreateReportProps {
    close?: () => void
    electionEventId: string | undefined
    tenantId: string | null
    isEditReport: boolean
    reportId?: Identifier | null | undefined
}

export enum EReportEncryption {
    UNENCRYPTED = "Unencrypted",
    CONFIGURED_PASSWORD = "Configured Password",
}

export const EditReportForm: React.FC<CreateReportProps> = ({
    close,
    tenantId,
    electionEventId,
    isEditReport,
    reportId,
}) => {
    const [reportType, setReportType] = useState<ETemplateType | undefined>(undefined)
    const [reportEncryptionPolicy, setReportEncryptionPolicy] = useState<
        EReportEncryption | undefined
    >(EReportEncryption.UNENCRYPTED)
    const [reportIsEncrypted, setReportIsEncrypted] = useState<boolean>(false)
    const [electionId, setElectionId] = useState<string | null | undefined>(undefined)
    const [templateId, setTemplateId] = useState<string | null | undefined>(undefined)
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
    const handleReportTypeChange = (event: any) => {
        setReportType(event.target.value)
    }
    const {t} = useTranslation()
    const notify = useNotify()

    const {
        data: report,
        isLoading,
        error,
    } = useGetOne<Sequent_Backend_Report>(
        "sequent_backend_report",
        {id: reportId},
        {enabled: isEditReport}
    )

    const reportTypeChoices = Object.values(EReportType).map((reportType) => ({
        id: reportType,
        name: t(`template.type.${reportType}`),
    }))

    const reportEncryptionPolicyChoices = Object.keys(EReportEncryption).map((key) => ({
        id: EReportEncryption[key as keyof typeof EReportEncryption],
        name: t(`reportsScreen.reportEncryptionPolicy.${key}`),
    }))

    useEffect(() => {
        setIsCronActive(report?.cron_config?.is_active || false)
        setReportType(report?.report_type ? (report.report_type as ETemplateType) : undefined)
        console.log({type: report?.report_type ? (report.report_type as ETemplateType) : ""})
    }, [report])

    useEffect(() => {
        //Reset the isCronActive state when the report type changes
        if (!canGenerateReportScheduled) {
            setIsCronActive(false)
        }
    }, [reportType])

    const handleSubmit = async (values: any) => {
        let cron_config_js: CronConfig = {}
        if (values.cron_config && isCronActive) {
            if (values.cron_config.is_active) {
                cron_config_js = {
                    isActive: values.cron_config.is_active,
                    cronExpression: values.cron_config.cron_expression,
                    emailRecipient: values.cron_config.email_recipients,
                }
            }
        }

        const formData: Partial<Sequent_Backend_Report> = {
            ...values,
            encryption_policy: values.encryption_policy,
            tenant_id: tenantId,
            election_event_id: electionEventId,
            cron_config: {
                is_active: cron_config_js.isActive,
                cron_expression: cron_config_js.cronExpression,
                email_recipients: cron_config_js.emailRecipient,
            },
        }

        try {
            console.log(isEditReport, "isEditReport")
            console.log(reportId, "reportId")

            if (isEditReport && reportId) {
                console.log({"update report": formData})

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
                console.log(reportData)

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
                setReportType(undefined)
                close()
            }
        } catch (error) {
            notify(t(`reportsScreen.messages.submitError`), {type: "error"})
        }
    }

    const handleCronToggle = (event: any) => {
        setIsCronActive(event.target.checked)
    }

    const isValidCron = (cron: string) => {
        console.log("cron", cron)
        const cronRegex =
            /^(\*|([0-5]?\d)|\*\/([0-5]?\d)) (\*|([0-5]?\d)|\*\/([0-5]?\d)) (\*|([01]?\d|2[0-3])|\*\/([01]?\d|2[0-3])) (\*|([1-9]|[12]\d|3[01])|\*\/([1-9]|[12]\d|3[01])) (\*|(0?[1-9]|1[0-2])|\*\/(0?[1-9]|1[0-2])) (\*|([0-6])|\*\/([0-6]))$/
        const isValid = cronRegex.test(cron)
        console.log("isValid", isValid)
        return isValid
    }

    const canGenerateReportScheduled = useMemo((): boolean => {
        if (!reportType) {
            return false
        }
        return reportTypeConfig[reportType].actions.some(
            (action) => action === ReportActions.GENERATE_SCHEDULED
        )
    }, [reportType])

    const isTemplateRequired = useMemo((): boolean => {
        if (!reportType) {
            return false
        }
        return reportTypeConfig[reportType].templateRequired ?? false
    }, [reportType])

    const isButtonDisabled = (): boolean => {
        return (
            (isTemplateRequired && !templateId) ||
            (electionPolicy === EReportElectionPolicy.ELECTION_REQUIRED && !electionId) ||
            (electionPolicy === EReportElectionPolicy.ELECTION_NOT_ALLOWED && !!electionId)
        )
    }

    const electionPolicy = useMemo((): EReportElectionPolicy => {
        if (!reportType) {
            return EReportElectionPolicy.ELECTION_ALLOWED
        }
        return reportTypeConfig[reportType].electionPolicy ?? EReportElectionPolicy.ELECTION_ALLOWED
    }, [reportType])

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

    console.log(report, "report")

    return (
        <>
            <Create hasEdit={isEditReport}>
                <SimpleForm
                    record={isEditReport ? report : undefined}
                    onSubmit={handleSubmit}
                    resource="sequent_backend_report"
                    toolbar={
                        <Toolbar>
                            <SaveButton disabled={isButtonDisabled()} />
                        </Toolbar>
                    }
                >
                    <Typography variant="h4">
                        {isEditReport
                            ? t("reportsScreen.edit.title")
                            : t("reportsScreen.create.title")}
                    </Typography>
                    <Typography variant="body2">
                        {" "}
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
                        disabled={electionPolicy == EReportElectionPolicy.ELECTION_NOT_ALLOWED}
                    />

                    <SelectTemplate
                        tenantId={tenantId}
                        templateType={
                            reportType
                                ? reportTypeConfig[reportType]?.associatedTemplateType
                                : undefined
                        }
                        source={"template_id"}
                        label={t("reportsScreen.fields.template")}
                        onSelectTemplate={(templateId) => setTemplateId(templateId)}
                        value={templateId}
                        isRequired={isTemplateRequired}
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

                    {/* <Button
                        label={"Encrypt"}
                        onClick={() => setHandlePasswordDialogOpen(!handlePasswordDialogOpen)}
                    /> */}

                    {canGenerateReportScheduled && (
                        <BooleanInput
                            source="cron_config.is_active"
                            label={t("reportsScreen.fields.repeatable")}
                            onChange={handleCronToggle}
                        />
                    )}

                    {isCronActive && (
                        <>
                            <TextInput
                                source="cron_config.cron_expression"
                                label={t("reportsScreen.fields.cronExpression")}
                                validate={(value) =>
                                    isValidCron(value) ? undefined : "Invalid cron expression"
                                }
                                required={isCronActive}
                            />
                            <TextInput
                                source="cron_config.email_recipients"
                                label={t("reportsScreen.fields.emailRecipients")}
                                required={isCronActive}
                            />
                        </>
                    )}
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
