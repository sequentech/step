// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"
import {
    Accordion,
    AccordionDetails,
    Box,
    Divider,
    LinearProgress,
    TableBody,
    TableRow,
} from "@mui/material"
import DownloadIcon from "@mui/icons-material/Download"
import {
    TransparentTable,
    TransparentTableCell,
    WidgetContainer,
    HeaderBox,
    InfoBox,
    TypeTypography,
    IconsBox,
    StyledIconButton,
    StyledProgressBar,
    LogTypography,
    LogsBox,
    CustomAccordionSummary,
    ViewTaskTypography,
    DownloaButton,
    StatusIconsBox,
} from "./styles/WidgetStyle"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import CloseIcon from "@mui/icons-material/Close"
import {Visibility} from "@mui/icons-material"
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import {ETasksExecution} from "@/types/tasksExecution"
import {StatusChip} from "./StatusChip"
import {IKeysCeremonyLog as ITaskLog} from "@/services/KeyCeremony"
import {useTranslation} from "react-i18next"
import {ViewTask} from "@/resources/Tasks/ViewTask"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {GET_TASK_BY_ID} from "@/queries/GetTaskById"
import {useQuery} from "@apollo/client"
import {DownloadDocument} from "@/resources/User/DownloadDocument"
import {Button} from "react-admin"
import {GetTaskByIdQuery} from "@/gql/graphql"

interface LogTableProps {
    logs: ITaskLog[]
    status: ETaskExecutionStatus
}

export const LogTable: React.FC<LogTableProps> = ({logs, status}) => {
    const isFailed = status === ETaskExecutionStatus.FAILED

    return (
        <TransparentTable className="logs-table">
            <TableBody>
                {logs.map((log, index) => (
                    <TableRow key={index}>
                        <TransparentTableCell
                            className="date-col"
                            sx={{paddingLeft: "0", width: "40%"}}
                        >
                            {new Date(log.created_date).toLocaleString()}
                        </TransparentTableCell>
                        <TransparentTableCell
                            className="log-text"
                            isFailed={isFailed && index === logs.length - 1}
                        >
                            {log.log_text}
                        </TransparentTableCell>
                    </TableRow>
                ))}
            </TableBody>
        </TransparentTable>
    )
}

export interface WidgetProps {
    identifier: string
    type: ETasksExecution
    status: ETaskExecutionStatus
    onClose: (taskId: string) => void
    onSuccess?: () => void
    onFailure?: () => void
    automaticallyDownload?: boolean
    logs?: Array<ITaskLog>
    taskId?: string
}

export const Widget: React.FC<WidgetProps> = ({
    identifier,
    type,
    status,
    onClose,
    onSuccess,
    onFailure,
    automaticallyDownload,
    logs,
    taskId,
}) => {
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const [expanded, setExpanded] = useState(false)
    const [openTaskModal, setOpenTaskModal] = useState(false)
    const [exportDocumentId, setExportDocumentId] = useState<string | undefined>(undefined)
    const [downloading, setDownloading] = useState<boolean>(false)
    const [taskDataType, setTaskDataType] = useState<ETasksExecution | undefined>(type)
    const [taskDataStatus, setTaskDataStatus] = useState<ETaskExecutionStatus>(status)
    const [taskDataLogs, setTaskDataLogs] = useState<Array<ITaskLog>>(logs || [])
    const [touchedDownload, setTouchedDownload] = useState(false)

    const initialLog: ITaskLog[] = [
        {created_date: new Date().toLocaleString(), log_text: "Task started"},
    ]

    const {data: taskData} = useQuery<GetTaskByIdQuery>(GET_TASK_BY_ID, {
        variables: {task_id: taskId},
        skip: !taskId,
        pollInterval: [ETaskExecutionStatus.STARTED, ETaskExecutionStatus.IN_PROGRESS].includes(
            taskDataStatus
        )
            ? globalSettings.QUERY_FAST_POLL_INTERVAL_MS
            : globalSettings.QUERY_POLL_INTERVAL_MS,
    })

    useEffect(() => {
        if (taskData && taskData.sequent_backend_tasks_execution.length > 0) {
            const task = taskData.sequent_backend_tasks_execution[0]
            setTaskDataType(task.type as ETasksExecution)
            setTaskDataStatus(task.execution_status as ETaskExecutionStatus)
            setTaskDataLogs(task.logs)
        }
    }, [taskData])

    useEffect(() => {
        if (taskDataStatus === ETaskExecutionStatus.FAILED) {
            setExpanded(true)
            onFailure && onFailure()
        } else if (taskDataStatus === ETaskExecutionStatus.SUCCESS) {
            onSuccess && onSuccess()
            if (touchedDownload) {
                return
            }
            const lastTask = taskData?.sequent_backend_tasks_execution?.[0]
            if (automaticallyDownload && lastTask?.annotations?.document_id) {
                setTouchedDownload(true)
                setDownloading(true)
                setExportDocumentId(lastTask?.annotations?.document_id)
            }
        }
    }, [
        taskDataStatus,
        downloading,
        touchedDownload,
        taskData?.sequent_backend_tasks_execution?.[0]?.annotations?.document_id,
        onFailure,
        onSuccess,
    ])

    const onSetViewTask = (event: React.MouseEvent<HTMLElement>) => {
        event.stopPropagation()
        setOpenTaskModal(!openTaskModal)
    }

    const lastTask = taskData?.sequent_backend_tasks_execution?.[0]

    return (
        <>
            <WidgetContainer className="widget-container">
                <Accordion
                    className="widget-accordion"
                    expanded={expanded}
                    onChange={() => setExpanded(!expanded)}
                >
                    <CustomAccordionSummary
                        className="accordion-summary"
                        isLoading={taskDataStatus === ETaskExecutionStatus.IN_PROGRESS}
                        expandIcon={<ExpandMoreIcon />}
                        sx={{backgroundColor: "#0F054C"}}
                    >
                        <HeaderBox className="header-box">
                            <InfoBox className="info-box">
                                <TypeTypography className="header">
                                    {t(`tasksScreen.widget.taskTitle`, {
                                        title: t(
                                            `tasksScreen.tasksExecution.${taskDataType || type}`
                                        ),
                                    })}
                                </TypeTypography>
                                <StatusIconsBox className="status-icons">
                                    <StatusChip status={taskDataStatus} />
                                    <IconsBox>
                                        {taskId ? (
                                            <StyledIconButton size="small" onClick={onSetViewTask}>
                                                <Visibility />
                                            </StyledIconButton>
                                        ) : null}
                                        <StyledIconButton
                                            size="small"
                                            onClick={() => onClose(identifier)}
                                        >
                                            <CloseIcon />
                                        </StyledIconButton>
                                    </IconsBox>
                                </StatusIconsBox>
                            </InfoBox>
                            {taskDataStatus === ETaskExecutionStatus.IN_PROGRESS && (
                                <StyledProgressBar>
                                    <LinearProgress />
                                </StyledProgressBar>
                            )}
                        </HeaderBox>
                    </CustomAccordionSummary>
                    <AccordionDetails
                        className="accordion-details"
                        sx={{display: "flex", flexDirection: "column", padding: "8px 16px"}}
                    >
                        <LogsBox className="logs-box">
                            <LogTypography className="logs-title">{t("widget.logs")}</LogTypography>
                            <Divider />
                            <LogTable
                                logs={taskDataLogs.length > 0 ? taskDataLogs : initialLog}
                                status={taskDataStatus || status}
                            />
                        </LogsBox>
                        <Box sx={{display: "flex", flexDirection: "row-reverse"}}>
                            {taskId ? (
                                <>
                                    <ViewTaskTypography
                                        className="view-icon"
                                        onClick={onSetViewTask}
                                    >
                                        {t("tasksScreen.widget.viewTask") as any}
                                    </ViewTaskTypography>
                                </>
                            ) : null}
                            {taskId &&
                            lastTask?.election_event_id &&
                            lastTask?.annotations?.document_id ? (
                                <DownloaButton
                                    onClick={() => {
                                        setDownloading(true)
                                        setExportDocumentId(lastTask?.annotations?.document_id)
                                    }}
                                    disabled={
                                        downloading ||
                                        lastTask?.execution_status !== ETaskExecutionStatus.SUCCESS
                                    }
                                    label={String(t("tasksScreen.widget.downloadDocument"))}
                                >
                                    <DownloadIcon />
                                </DownloaButton>
                            ) : null}
                        </Box>
                    </AccordionDetails>
                </Accordion>
            </WidgetContainer>

            {openTaskModal && taskId && (
                <ViewTask
                    currTaskId={taskId}
                    goBack={() => setOpenTaskModal(false)}
                    isModal={true}
                />
            )}
            {exportDocumentId && downloading && (
                <>
                    <DownloadDocument
                        documentId={(downloading && exportDocumentId) || ""}
                        electionEventId={lastTask?.election_event_id ?? ""}
                        fileName={null}
                        onDownload={() => {
                            setDownloading(false)
                            setExportDocumentId(undefined)
                        }}
                    />
                </>
            )}
        </>
    )
}
