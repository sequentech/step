import SelectElection from "@/components/election/SelectElection"
import {EReportTypes} from "@/types/reports"
import {Typography} from "@mui/material"
import React, {useEffect, useState} from "react"
import {
    BooleanInput,
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

interface CronConfig {
    isActive?: boolean
    cronExpression?: string
    emailRecepient?: string
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
    const [reportType, setReportType] = useState<EReportTypes>(EReportTypes.ELECTORAL_RESULTS)
    const [electionId, setElectionId] = useState<string | null | undefined>(undefined)
    const [templateId, setTemplateId] = useState<string | null | undefined>(undefined)
    const [createReport] = useMutation(CREATE_REPORT)
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
    const reportTypeChoices = Object.values(EReportTypes).map((reportType) => ({
        id: reportType,
        name: reportType,
    }))
    useEffect(() => {
        console.log("report", report)
        setIsCronActive(report?.cron_config?.is_active || false)
    }, [report])

    const handleSubmit = async (values: any) => {
        let cron_conig_js: CronConfig = {}
        if (values.cron_config) {
            if (values.cron_config.is_active) {
                cron_conig_js = {
                    isActive: values.cron_config.is_active,
                    cronExpression: values.cron_config.cron_expression,
                    emailRecepient: values.cron_config.email_recepient,
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
                email_recepient: cron_conig_js.emailRecepient,
            },
        }

        try {
            if (isEditReport && reportId) {
                await dataProvider.update("sequent_backend_report", {
                    id: reportId,
                    data: formData,
                    previousData: undefined,
                })
                notify("Report updated successfully", {type: "success"})
            } else {
                await createReport({
                    variables: {
                        object: formData,
                    },
                })
                notify("Report created successfully", {type: "success"})
            }

            if (close) {
                close()
            }
        } catch (error) {
            notify("Error submitting report", {type: "error"})
        }
    }

    const handleCronToggle = (event: any) => {
        setIsCronActive(event.target.checked)
    }

    const isValidCron = (cron: string) => {
        console.log("cron", cron)
        const cronRegex =
            /^(\*|([0-9]|1[0-9]|2[0-9]|3[0-9]|4[0-9]|5[0-9])|\*\/([0-9]|1[0-9]|2[0-9]|3[0-9]|4[0-9]|5[0-9])) (\*|([0-9]|1[0-9]|2[0-3])|\*\/([0-9]|1[0-9]|2[0-3])) (\*|([1-9]|1[0-9]|2[0-9]|3[0-1])|\*\/([1-9]|1[0-9]|2[0-9]|3[0-1])) (\*|([1-9]|1[0-2])|\*\/([1-9]|1[0-2])) (\*|([0-6])|\*\/([0-6]))$/
        const isValid = cronRegex.test(cron)
        console.log("isValid", isValid)
        return isValid
    }

    return (
        <>
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
                    <Typography variant="h4">
                        {isEditReport
                            ? t("reportsScreen.edit.header")
                            : t("reportsScreen.create.header")}
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
                    />

                    <SelectTemplate
                        tenantId={tenantId}
                        templateType={reportType}
                        source={"template_id"}
                        label={t("reportsScreen.fields.template")}
                        onSelectTemplate={(templateId) => setTemplateId(templateId)}
                        value={templateId}
                    />

                    <BooleanInput
                        source="cron_config.is_active"
                        label={t("reportsScreen.fields.repeatable")}
                        onChange={handleCronToggle}
                    />

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
                                source="cron_config.email_recepient"
                                label={t("reportsScreen.fields.cronExpression")}
                                required={isCronActive}
                            />
                        </>
                    )}
                </SimpleForm>
            </Create>
        </>
    )
}
