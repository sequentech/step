// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import SelectElection from "@/components/election/SelectElection"
import {EReportElectionPolicy, EReportType, ReportActions, reportTypeConfig} from "@/types/reports"
import {Box, IconButton, TextField, Tooltip, Typography} from "@mui/material"
import React, {useEffect, useMemo, useState} from "react"
import {
    BooleanInput,
    Button,
    Create,
    Identifier,
    SaveButton,
    SelectInput,
    SimpleForm,
    TextInput,
    Toolbar,
    useDataProvider,
    useGetOne,
    useNotify,
} from "react-admin"
import SelectTemplate from "../Template/SelectTemplate"
import {useTranslation} from "react-i18next"
import {Sequent_Backend_Report} from "@/gql/graphql"
import {useMutation} from "@apollo/client"
import {CREATE_REPORT} from "@/queries/CreateReport"
import {UPDATE_REPORT} from "@/queries/UpdateReport"
import {Dialog} from "@sequentech/ui-essentials"

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

export const EditReportForm: React.FC<CreateReportProps> = ({
    close,
    tenantId,
    electionEventId,
    isEditReport,
    reportId,
}) => {
    const [reportType, setReportType] = useState<EReportType | undefined>(undefined)
    const [electionId, setElectionId] = useState<string | null | undefined>(undefined)
    const [templateId, setTemplateId] = useState<string | null | undefined>(undefined)
    const [createReport] = useMutation(CREATE_REPORT)
    const [updateReport] = useMutation(UPDATE_REPORT)
    const [handlePasswordDialogOpen, setHandlePasswordDialogOpen] = useState<boolean>(false)
    const [filePassord, setFilePassword] = useState({password: "", confirmPassword: ""})
    const [isCronActive, setIsCronActive] = useState<boolean>(false)
    const dataProvider = useDataProvider()
    const handleReportTypeChange = (event: any) => {
        setReportType(event.target.value)
    }
    const {t} = useTranslation()
    const notify = useNotify()
    useEffect(() => {
        console.log("isCronActive", isCronActive)
    }, [])
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
        name: t(`reportsScreen.reportType.${reportType}`),
    }))
    useEffect(() => {
        setIsCronActive(report?.cron_config?.is_active || false)
        setReportType(report?.report_type ? (report.report_type as EReportType) : undefined)
    }, [report])

    useEffect(() => {
        //Reset the isCronActive state when the report type changes
        if (!canGenerateReportSchedulued) {
            setIsCronActive(false)
        }
    }, [reportType])

    const handleSubmit = async (values: any) => {
        let cron_conig_js: CronConfig = {}
        if (values.cron_config && isCronActive) {
            if (values.cron_config.is_active) {
                cron_conig_js = {
                    isActive: values.cron_config.is_active,
                    cronExpression: values.cron_config.cron_expression,
                    emailRecipient: values.cron_config.email_recipients,
                }
            }
        }

        const formData = {
            ...values,
            tenant_id: tenantId,
            election_event_id: electionEventId,
            cron_config: {
                is_active: cron_conig_js.isActive,
                cron_expression: cron_conig_js.cronExpression,
                email_recipients: cron_conig_js.emailRecipient,
            },
        }

        try {
            if (isEditReport && reportId) {
                await updateReport({
                    variables: {
                        id: reportId,
                        set: formData,
                    },
                })
                notify(t(`reportsScreen.messages.updateSuccess`), {type: "success"})
            } else {
                await createReport({
                    variables: {
                        object: formData,
                    },
                })
                notify(t(`reportsScreen.messages.createSuccess`), {type: "success"})
            }

            if (close) {
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

    const canGenerateReportSchedulued = useMemo((): boolean => {
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

    const PasswordDialog: React.FC<{password: string; onClose: () => void}> = ({
        password,
        onClose,
    }) => {
        const {t} = useTranslation()
        const notify = useNotify()

        return (
            <Dialog
                variant="info" 
                open={true}
                handleClose={onClose}
                aria-labelledby="password-dialog-title"
                title={t("electionEventScreen.export.passwordTitle")}
                ok={"Ok"}
            >
                <Box component={"form"}>
                    {"Password"}
                    <TextField
                        fullWidth
                        margin="normal"
                        type="password"
                        value={filePassord.password}
                        onChange={(e) =>
                            setFilePassword({
                                password: e.target.value,
                                confirmPassword: filePassord.confirmPassword,
                            })
                        }
                    />
                    {"Confirm password"}
                    <TextField
                        fullWidth
                        margin="normal"
                        type="password"
                        value={filePassord.confirmPassword}
                        onChange={(e) =>
                            setFilePassword({
                                password: filePassord.password,
                                confirmPassword: e.target.value,
                            })
                        }
                    />
                </Box>
            </Dialog>
        )
    }

    return (
        <>
            <Create hasEdit={isEditReport}>
                <SimpleForm
                    record={isEditReport ? report : undefined}
                    onSubmit={handleSubmit}
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
                        label={t("reportsScreen.fields.reportType")}
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

                    <Button
                        label={"Encrypt"}
                        onClick={() => setHandlePasswordDialogOpen(!handlePasswordDialogOpen)}
                    />

                    {canGenerateReportSchedulued && (
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
            {handlePasswordDialogOpen && (
                <PasswordDialog onClose={() => setHandlePasswordDialogOpen(false)} password="" />
            )}
        </>
    )
}
