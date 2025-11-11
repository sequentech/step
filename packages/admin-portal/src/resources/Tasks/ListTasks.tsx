// SPDX-FileCopyrightText: 2024 Félix Robles <dev@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useEffect, useState} from "react"
import {
    List,
    TextInput,
    DateField,
    FunctionField,
    TextField,
    DatagridConfigurable,
    useNotify,
    Identifier,
} from "react-admin"
import {useTranslation} from "react-i18next"
import {ExportTasksExecutionMutation, Sequent_Backend_Election_Event} from "@/gql/graphql"
import {Visibility} from "@mui/icons-material"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {ListActions} from "@/components/ListActions"
import {StatusChip} from "@/components/StatusChip"
import {useMutation} from "@apollo/client"
import {EXPORT_TASKS_EXECUTION} from "@/queries/ExportTasksExecution"
import {FormStyles} from "@/components/styles/FormStyles"
import {DownloadDocument} from "../User/DownloadDocument"
import {Dialog} from "@sequentech/ui-essentials"
import {IPermissions} from "@/types/keycloak"
import {ResetFilters} from "@/components/ResetFilters"
import {useTasksPermissions} from "./useTasksPermissions"
import {CircularProgress} from "@mui/material"

export interface ListTasksProps {
    onViewTask: (id: Identifier) => void
    electionEventRecord?: Sequent_Backend_Election_Event
}
export const ListTasks: React.FC<ListTasksProps> = ({onViewTask, electionEventRecord}) => {
    const notify = useNotify()
    const {t} = useTranslation()
    const [openExport, setOpenExport] = useState(false)
    const [exporting, setExporting] = useState(false)

    const {canReadTasks, canExportTasks, showTasksColumns, showTasksFilters, showTasksBackButton} =
        useTasksPermissions()

    const [exportDocumentId, setExportDocumentId] = useState<string | undefined>()
    const [exportTasksExecution] = useMutation<ExportTasksExecutionMutation>(
        EXPORT_TASKS_EXECUTION,
        {
            context: {
                headers: {
                    "x-hasura-role": IPermissions.TASKS_EXPORT,
                },
            },
        }
    )

    const OMIT_FIELDS: string[] = []

    const filters: Array<ReactElement> = [
        <TextInput source="id" key="id_filter" label={String(t("tasksScreen.column.id"))} />,
        <TextInput source="type" key="type_filter" label={String(t("tasksScreen.column.type"))} />,
        <TextInput
            source="execution_status"
            key="status_filter"
            label={String(t("tasksScreen.column.execution_status"))}
        />,
    ]

    const actions: Action[] = [
        {
            icon: <Visibility />,
            action: onViewTask,
            showAction: () => canReadTasks,
        },
    ]

    const handleExport = () => {
        setExporting(false)
        setExportDocumentId(undefined)
        setOpenExport(true)
    }

    const confirmExportAction = async () => {
        if (!electionEventRecord) {
            notify(t("tasksScreen.exportTasksExecution.error"))
            setOpenExport(false)
            return
        }

        try {
            setExporting(true)
            const {data: exportTasksExecutionData, errors} = await exportTasksExecution({
                variables: {
                    tenantId: electionEventRecord.tenant_id,
                    electionEventId: electionEventRecord.id,
                },
            })
            if (errors || !exportTasksExecutionData) {
                setExporting(false)
                setOpenExport(false)
                notify(t("tasksScreen.exportTasksExecution.error"))
                return
            }
            let documentId = exportTasksExecutionData.export_tasks_execution?.document_id
            setExportDocumentId(documentId)
        } catch (err) {
            setExporting(false)
            setOpenExport(false)
            notify(t("tasksScreen.exportTasksExecution.error"))
            console.log(err)
        }
    }

    if (!electionEventRecord) {
        return <CircularProgress />
    }

    return (
        <>
            <List
                actions={
                    <ListActions
                        withColumns={showTasksColumns}
                        withFilter={showTasksFilters}
                        withImport={false}
                        doExport={handleExport}
                        withExport={canExportTasks}
                    />
                }
                resource="sequent_backend_tasks_execution"
                filters={filters}
                filter={{election_event_id: electionEventRecord?.id || undefined}}
                storeKey={false}
                sort={{field: "start_at", order: "DESC"}}
                perPage={10}
                disableSyncWithLocation
            >
                <ResetFilters />
                <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={false}>
                    <TextField source="id" />
                    <TextField source="name" />
                    <DateField
                        source="start_at"
                        showTime={true}
                        label={String(t("tasksScreen.column.start_at"))}
                    />
                    <FunctionField
                        label={String(t("tasksScreen.column.execution_status"))}
                        render={(record: any) => <StatusChip status={record.execution_status} />}
                    />
                    <ActionsColumn actions={actions} label={String(t("common.label.actions"))} />
                </DatagridConfigurable>
            </List>

            <Dialog
                variant="info"
                open={openExport}
                ok={String(t("common.label.export"))}
                okEnabled={() => !exporting}
                cancel={String(t("common.label.cancel"))}
                title={String(t("common.label.export"))}
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
                            fileName={`export-tasks-execution.json`}
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
        </>
    )
}
