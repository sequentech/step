// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useEffect, useState} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    FunctionField,
    NumberField,
    useRecordContext,
    useNotify,
    useListController,
    TextInput,
    DateInput,
    DateField,
    DateTimeInput,
} from "react-admin"
import {ListActions} from "@/components/ListActions"
import {useTranslation} from "react-i18next"
import {Sequent_Backend_Election, Sequent_Backend_Election_Event} from "@/gql/graphql"
import {Dialog} from "@sequentech/ui-essentials"
import {FormStyles} from "./styles/FormStyles"
import {DownloadDocument} from "@/resources/User/DownloadDocument"
import {EXPORT_ELECTION_EVENT_LOGS} from "@/queries/ExportElectionEventLogs"
import {useMutation} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {useLocation, useNavigate} from "react-router"
import {ResetFilters} from "./ResetFilters"
import {MenuItem, Menu} from "@mui/material"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import {useLogsPermissions} from "@/resources/ElectionEvent/useLogsPermissions"
import {MessageField} from "./MessageField"

enum ExportFormat {
    CSV = "CSV",

    // turns out that the pdf is zipped
    PDF = "PDF",
}

const OMIT_FIELDS = ["user_id"]

interface ExportWrapperProps {
    electionEventId: string
    openExport: boolean
    setOpenExport: (val: boolean) => void
    exportFormat: string
}
const ExportDialog: React.FC<ExportWrapperProps> = ({
    electionEventId,
    openExport,
    setOpenExport,
    exportFormat,
}) => {
    const {t} = useTranslation()
    const [exportDocumentId, setExportDocumentId] = React.useState<string | undefined>(undefined)
    const [exportElectionEventActivityLogs] = useMutation(EXPORT_ELECTION_EVENT_LOGS, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.LOGS_EXPORT,
            },
        },
    })
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const download = async () => {
        const currWidget = addWidget(ETasksExecution.EXPORT_ACTIVITY_LOGS_REPORT, undefined)
        try {
            const {data: exportElectionEventData, errors} = await exportElectionEventActivityLogs({
                variables: {
                    electionEventId,
                    format: exportFormat,
                },
            })
            if (errors) {
                updateWidgetFail(currWidget.identifier)
                return
            }
            let documentId = exportElectionEventData?.export_election_event_logs?.document_id
            setExportDocumentId(documentId)
            const task_id = exportElectionEventData?.export_election_event_logs?.task_execution.id
            setWidgetTaskId(currWidget.identifier, task_id)
        } catch (error) {
            updateWidgetFail(currWidget.identifier)
            setExportDocumentId(undefined)
        }
    }
    const confirmExportAction = () => {
        setOpenExport(false)
        download()
    }

    return (
        <>
            <Dialog
                variant="info"
                open={openExport}
                ok={String(t("common.label.export"))}
                cancel={String(t("common.label.cancel"))}
                title={String(t("common.label.exportFormat", {
                    item: t("logsScreen.title"),
                    format: exportFormat,
                }))}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmExportAction()
                    } else {
                        setOpenExport(false)
                    }
                }}
            >
                <span>{t("logsScreen.exportdialog.description")}</span>
            </Dialog>
            {exportDocumentId && (
                <>
                    <DownloadDocument
                        documentId={exportDocumentId ?? ""}
                        electionEventId={electionEventId}
                        fileName={null}
                        onDownload={() => {
                            setExportDocumentId(undefined)
                        }}
                    />
                </>
            )}
        </>
    )
}

export interface ElectoralLogListProps {
    aside?: ReactElement
    filterToShow?: ElectoralLogFilters
    filterValue?: string
    electionEventId?: string
    showActions?: boolean
}

export enum ElectoralLogFilters {
    ID = "id",
    STATEMENT_KIND = "statement_kind",
    USER_ID = "user_id",
    USERNAME = "username",
}

export const ElectoralLogList: React.FC<ElectoralLogListProps> = ({
    aside,
    filterToShow,
    filterValue,
    electionEventId,
    showActions = true,
}) => {
    const record = useRecordContext<Sequent_Backend_Election_Event | Sequent_Backend_Election>()
    const {t} = useTranslation()

    const {canExportLogs, showLogsColumns} = useLogsPermissions()

    const getHeadField = (record: any, field: string) => {
        const message = JSON.parse(record?.message)
        if (
            !message ||
            !message.statement ||
            !message.statement.head ||
            !message.statement.head[field]
        ) {
            return <span>-</span>
        }
        return message.statement.head[field]
    }

    const [openExport, setOpenExport] = React.useState(false)
    const [exportFormat, setExportFormat] = React.useState(ExportFormat.CSV)

    const handleExportWithOptions = (format: ExportFormat) => {
        setExportFormat(format)
        setOpenExport(true)
        setAnchorEl(null)
    }

    const filterObject: {[key: string]: any} = {
        election_event_id: electionEventId || record?.id || undefined,
    }

    if (filterToShow) {
        filterObject[filterToShow] = filterValue || undefined
    }

    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)

    const filters: Array<ReactElement> = [
        <TextInput key={"user_id"} source={"user_id"} label={String(t("logsScreen.column.user_id"))} />,
        <TextInput key={"username"} source={"username"} label={String(t("logsScreen.column.username"))} />,
        <DateTimeInput key={"created"} source={"created"} label={String(t("logsScreen.column.created"))} />,
        <DateTimeInput
            key={"statement_timestamp"}
            source={"statement_timestamp"}
            label={String(t("logsScreen.column.statement_timestamp"))}
        />,
        <TextInput
            key={"statement_kind"}
            source={"statement_kind"}
            label={String(t("logsScreen.column.statement_kind"))}
        />,
    ]

    return (
        <>
            <List
                resource="electoral_log"
                actions={
                    showActions && (
                        <ListActions
                            withColumns={showLogsColumns}
                            withImport={false}
                            openExportMenu={(e) => setAnchorEl(e.currentTarget)}
                            withExport={canExportLogs}
                            withFilter={true}
                        />
                    )
                }
                filters={filters}
                filter={filterObject}
                storeKey={false}
                sort={{
                    field: "id",
                    order: "DESC",
                }}
                aside={aside}
            >
                <ResetFilters />
                <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={false}>
                    <NumberField source="id" label={String(t("logsScreen.column.id"))} />
                    <FunctionField
                        source="user_id"
                        label={String(t("logsScreen.column.user_id"))}
                        render={(record: any) => {
                            const userId = JSON.parse(record.message).user_id
                            return (
                                <span style={{display: "block", textAlign: "center"}}>
                                    {!userId || userId === "null" ? <span>-</span> : userId}
                                </span>
                            )
                        }}
                    />
                    <FunctionField
                        source="username"
                        label={String(t("logsScreen.column.username"))}
                        render={(record: any) => {
                            const username = JSON.parse(record.message).username
                            return (
                                <span style={{display: "block", textAlign: "center"}}>
                                    {!username || username === "null" ? <span>-</span> : username}
                                </span>
                            )
                        }}
                    />
                    <FunctionField
                        source="created"
                        label={String(t("logsScreen.column.created"))}
                        render={(record: any) => new Date(record.created * 1000).toUTCString()}
                    />
                    <FunctionField
                        source="statement_timestamp"
                        label={String(t("logsScreen.column.statement_timestamp"))}
                        render={(record: any) =>
                            new Date(record.statement_timestamp * 1000).toUTCString()
                        }
                    />
                    <TextField source="statement_kind" />
                    <FunctionField
                        source="event_type"
                        label={String(t("logsScreen.column.statement_kind"))}
                        render={(record: any) => getHeadField(record, "event_type")}
                    />
                    <FunctionField
                        source="log_type"
                        label={String(t("logsScreen.column.log_type"))}
                        render={(record: any) => getHeadField(record, "log_type")}
                    />
                    <FunctionField
                        source="description"
                        label={String(t("logsScreen.column.description"))}
                        render={(record: any) => (
                            <MessageField
                                content={getHeadField(record, "description")}
                                initialLength={50}
                            />
                        )}
                    />
                    <MessageField source="message" />
                </DatagridConfigurable>
            </List>
            <ExportDialog
                electionEventId={record?.id ?? ""}
                openExport={openExport}
                setOpenExport={setOpenExport}
                exportFormat={exportFormat}
            />
            <Menu
                id="menu-export-logs"
                anchorEl={anchorEl}
                anchorOrigin={{
                    vertical: "bottom",
                    horizontal: "right",
                }}
                keepMounted
                transformOrigin={{
                    vertical: "top",
                    horizontal: "right",
                }}
                open={Boolean(anchorEl)}
                onClose={() => setAnchorEl(null)}
            >
                <MenuItem
                    className="menu-export-csv"
                    onClick={() => handleExportWithOptions(ExportFormat.CSV)}
                >
                    <span className="help-menu-item-CSV">{t(`logsScreen.actions.csv`)}</span>
                </MenuItem>
                <MenuItem
                    className="menu-export-pdf"
                    onClick={() => handleExportWithOptions(ExportFormat.PDF)}
                >
                    <span className="help-menu-item-PDF">{t(`logsScreen.actions.pdf`)}</span>
                </MenuItem>
            </Menu>
        </>
    )
}
