// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useEffect, useState } from "react"
import {
    List,
    DateField,
    FunctionField,
    TextField,
    DatagridConfigurable,
    Identifier,
    SelectInput,
    TextInput,
    useListContext,
    DatagridConfigurableProps,
    useNotify,
    useRefresh,
} from "react-admin"
import { TFunction, useTranslation } from "react-i18next"
import { Visibility } from "@mui/icons-material"
import { Action, ActionsColumn } from "@/components/ActionButons"
import { ListActions } from "@/components/ListActions"
import {
    ExportApplicationMutation,
    ImportApplicationMutation,
    Sequent_Backend_Election_Event,
} from "@/gql/graphql"
import { StatusApplicationChip } from "@/components/StatusApplicationChip"
import { Dialog } from "@sequentech/ui-essentials"
import { FormStyles } from "@/components/styles/FormStyles"
import { DownloadDocument } from "../User/DownloadDocument"
import { useMutation } from "@apollo/client"
import { EXPORT_APPLICATION } from "@/queries/ExportApplication"
import { IPermissions } from "@/types/keycloak"
import { WidgetProps } from "@/components/Widget"
import { ETasksExecution } from "@/types/tasksExecution"
import { useWidgetStore } from "@/providers/WidgetsContextProvider"
import { ImportDataDrawer } from "@/components/election-event/import-data/ImportDataDrawer"
import { IMPORT_APPLICATION } from "@/queries/ImportApplication"

export interface ListApprovalsProps {
    electionEventId: string
    electionId?: string
    onViewApproval: (id: Identifier) => void
    electionEventRecord: Sequent_Backend_Election_Event
}

interface ApprovalsListProps extends Omit<DatagridConfigurableProps, "children"> {
    omit: string[]
    actions: Action[]
    t: TFunction
}

// Storage key for the status filter
const STATUS_FILTER_KEY = "approvals_status_filter"

const ApprovalsList = (props: ApprovalsListProps) => {
    const { filterValues } = useListContext()

    // Monitor and save filter changes
    useEffect(() => {
        if (filterValues?.status) {
            localStorage.setItem(STATUS_FILTER_KEY, filterValues.status)
        }
    }, [filterValues?.status])

    return (
        <div>
            <DatagridConfigurable {...props} omit={props.omit} bulkActionButtons={false}>
                <TextField source="id" />
                <DateField showTime source="created_at" />
                <DateField showTime source="updated_at" />
                <TextField source="applicant_id" />
                <TextField source="verification_type" />
                <FunctionField
                    label={props.t("approvalsScreen.column.status")}
                    render={(record: any) => (
                        <StatusApplicationChip status={record.status.toUpperCase()} />
                    )}
                />
                <ActionsColumn actions={props.actions} label={props.t("common.label.actions")} />
            </DatagridConfigurable>
        </div>
    )
}

const CustomFilters = () => {
    const { t } = useTranslation()

    return [
        <SelectInput
            source="status"
            key="status_filter"
            label={t("approvalsScreen.column.status")}
            choices={[
                { id: "pending", name: "Pending" },
                { id: "accepted", name: "Accepted" },
                { id: "rejected", name: "Rejected" },
            ]}
        />,
        <SelectInput
            source="verification_type"
            key="verification_type_filter"
            label={t("approvalsScreen.column.verificationType")}
            choices={[
                { id: "MANUAL", name: "Manual" },
                { id: "AUTOMATIC", name: "Automatic" },
            ]}
        />,
        <TextInput
            key={"applicant_id_filter"}
            source="applicant_id"
            label={t("approvalsScreen.column.applicantId")}
        />,
        <TextInput key={"id_filter"} source="id" label={t("approvalsScreen.column.id")} />,
    ]
}

export const ListApprovals: React.FC<ListApprovalsProps> = ({
    electionEventId,
    electionId,
    onViewApproval,
    electionEventRecord,
}) => {
    const { t } = useTranslation()
    const OMIT_FIELDS: string[] = []
    const [openExport, setOpenExport] = useState(false)
    const [exporting, setExporting] = useState(false)
    const [exportDocumentId, setExportDocumentId] = useState<string | undefined>()
    const notify = useNotify()
    const [openImportDrawer, setOpenImportDrawer] = useState<boolean>(false)
    const refresh = useRefresh()
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const [exportApplication] = useMutation<ExportApplicationMutation>(EXPORT_APPLICATION, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.APPLICATION_EXPORT,
            },
        },
    })
    const [importApplications] = useMutation<ImportApplicationMutation>(IMPORT_APPLICATION, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.APPLICATION_IMPORT,
            },
        },
    })

    const handleExport = () => {
        setExporting(false)
        setExportDocumentId(undefined)
        setOpenExport(true)
    }

    const handleImport = () => {
        setOpenImportDrawer(true)
    }

    const handleImportApplications = async (documentId: string, sha256: string) => {
        setOpenImportDrawer(false)
        try {
            await importApplications({
                variables: {
                    tenantId: electionEventRecord.tenant_id,
                    electionEventId: electionEventRecord.id,
                    electionId: electionId,
                    documentId,
                },
            })
            notify("Templates imported successfully", { type: "success" })
            refresh()
        } catch (err) {
            console.log(err)
            notify("Error importing templates", { type: "error" })
        }
    }

    const confirmExportAction = async () => {
        if (!electionEventRecord) {
            notify(t("tasksScreen.exportApplication.error"))
            setOpenExport(false)
            return
        }
        let currWidget: WidgetProps | undefined
        try {
            setExporting(true)
            currWidget = addWidget(ETasksExecution.EXPORT_APPLICATION)
            const { data: exportApplicationData, errors } = await exportApplication({
                variables: {
                    tenantId: electionEventRecord.tenant_id,
                    electionEventId: electionEventRecord.id,
                    electionId: electionId,
                },
            })

            if (errors || !exportApplicationData) {
                setExporting(false)
                setOpenExport(false)
                notify(t("tasksScreen.exportTasksExecution.error"))
                updateWidgetFail(currWidget.identifier)
                return
            }
            let documentId = exportApplicationData.export_application?.document_id
            const task_id = exportApplicationData?.export_application?.task_execution?.id
            setExportDocumentId(documentId)
            task_id
                ? setWidgetTaskId(currWidget.identifier, task_id)
                : updateWidgetFail(currWidget.identifier)
        } catch (err) {
            setExporting(false)
            setOpenExport(false)
            currWidget && updateWidgetFail(currWidget.identifier)
            console.log(err)
        }
    }

    const actions: Action[] = [
        {
            icon: <Visibility />,
            action: onViewApproval,
        },
    ]

    // Get initial status from localStorage or use "pending" as default
    const initialStatus = localStorage.getItem(STATUS_FILTER_KEY) || "pending"

    return (
        <>
            <List
                actions={
                    <ListActions
                        withImport={true}
                        withExport={true}
                        doImport={handleImport}
                        doExport={handleExport}
                    />
                }
                resource="sequent_backend_applications"
                filters={CustomFilters()}
                filter={{ election_event_id: electionEventId || undefined }}
                sort={{ field: "created_at", order: "DESC" }}
                perPage={10}
                filterDefaultValues={{ status: initialStatus }}
                disableSyncWithLocation
                storeKey="approvals-list"
            >
                <ApprovalsList omit={OMIT_FIELDS} actions={actions} t={t} />
            </List>
            <Dialog
                variant="info"
                open={openExport}
                ok={t("common.label.export")}
                okEnabled={() => !exporting}
                cancel={t("common.label.cancel")}
                title={t("common.label.export")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmExportAction()
                    } else {
                        setExportDocumentId(undefined)
                        setExporting(false)
                        setOpenExport(false)
                    }
                }}
            >
                {t("common.export")}
                <FormStyles.ReservedProgressSpace>
                    {exporting ? <FormStyles.ShowProgress /> : null}
                    {exporting && exportDocumentId ? (
                        <DownloadDocument
                            documentId={exportDocumentId}
                            electionEventId={electionEventRecord?.id || ""}
                            fileName={`export-applications.csv`}
                            onDownload={() => {
                                console.log("onDownload called")
                                setExportDocumentId(undefined)
                                setExporting(false)
                                setOpenExport(false)
                                notify(t("tasksScreen.exportTasksExecution.success"), {
                                    type: "success",
                                })
                            }}
                        />
                    ) : null}
                </FormStyles.ReservedProgressSpace>
            </Dialog>

            <ImportDataDrawer
                open={openImportDrawer}
                closeDrawer={() => setOpenImportDrawer(false)}
                title="template.import.title"
                subtitle="template.import.subtitle"
                paragraph="template.import.paragraph"
                doImport={handleImportApplications}
                errors={null}
            />
        </>
    )
}
