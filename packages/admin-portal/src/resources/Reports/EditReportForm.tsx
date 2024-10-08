import SelectElection from "@/components/election/SelectElection"
import {EReportTypes} from "@/types/reports"
import {Typography} from "@mui/material"
import {t} from "i18next"
import React, {useEffect, useState} from "react"
import {
    CreateBase,
    Identifier,
    RaRecord,
    RecordContext,
    SaveButton,
    SelectInput,
    SimpleForm,
    Toolbar,
    useDataProvider,
    useGetOne,
    useListContext,
    useNotify,
} from "react-admin"
import SelectTemplate from "../Template/SelectTemplate"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import { useTranslation } from "react-i18next"

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
    const dataProvider = useDataProvider()
    const handleReportTypeChange = (event: any) => {
        setReportType(event.target.value)
    }
    const {t} = useTranslation();
    const notify = useNotify()

    const { data: report, isLoading, error } = useGetOne(
        "sequent_backend_report",
        { id: reportId },
        { enabled: isEditReport }
    );
    const reportTypeChoices = Object.values(EReportTypes).map((reportType) => ({
        id: reportType,
        name: reportType,
    }))

    const handleSubmit = async (values: any) => {
        const formData = {
            ...values,
            tenant_id: tenantId,
            election_event_id: electionEventId,
        }

        try {
            if (isEditReport && reportId) {
                await dataProvider.update("sequent_backend_report", {
                    id: reportId,
                    data: formData,
                    previousData: undefined
                });
                notify('Report updated successfully', { type: 'success' });
            } else {
                await dataProvider.create("sequent_backend_report", { data: formData });
                notify('Report created successfully', { type: 'success' });
            }

            if (close) {
                close();
            }
        } catch (error) {
            notify('Error submitting report', { type: 'error' });
        }
    };

    return (
        <>
            <SimpleForm
            record={isEditReport ? report : undefined}
                onSubmit={handleSubmit}
                toolbar={
                    <Toolbar>
                        <SaveButton />
                    </Toolbar>
                }
            >
                <Typography variant="h4">{
                    isEditReport ? t("reportsScreen.edit.header") : t("reportsScreen.create.header")
                    }</Typography>
                <Typography variant="body2"> {
                    isEditReport ? t("reportsScreen.edit.subtitle") : t("reportsScreen.create.subtitle")
                    }</Typography>

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
            </SimpleForm>
        </>
    )
}

