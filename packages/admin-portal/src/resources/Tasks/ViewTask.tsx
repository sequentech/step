// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {GetTaskByIdQuery, Sequent_Backend_Election_Event} from "@/gql/graphql"
import React, {useContext, useState} from "react"
import DownloadIcon from "@mui/icons-material/Download"
import {useTranslation} from "react-i18next"
import {theme, Dialog} from "@sequentech/ui-essentials"
import {WizardStyles} from "@/components/styles/WizardStyles"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import {Accordion, AccordionSummary, CircularProgress} from "@mui/material"
import Table from "@mui/material/Table"
import TableBody from "@mui/material/TableBody"
import TableCell from "@mui/material/TableCell"
import TableContainer from "@mui/material/TableContainer"
import TableRow from "@mui/material/TableRow"
import Paper from "@mui/material/Paper"
import {Button, Identifier} from "react-admin"
import {Logs} from "@/components/Logs"
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useQuery} from "@apollo/client"
import {GET_TASK_BY_ID} from "@/queries/GetTaskById"
import {CancelButton} from "../Tally/styles"
import {useTasksPermissions} from "./useTasksPermissions"
import {DownloadDocument} from "../User/DownloadDocument"
import {DownloaButton} from "@/components/styles/WidgetStyle"

export const statusColor: (status: string) => string = (status) => {
    if (status === ETaskExecutionStatus.STARTED) {
        return theme.palette.warning.light
    } else if (status === ETaskExecutionStatus.IN_PROGRESS) {
        return theme.palette.info.main
    } else if (status === ETaskExecutionStatus.SUCCESS) {
        return theme.palette.brandSuccess
    } else if (status === ETaskExecutionStatus.CANCELLED) {
        return theme.palette.errorColor
    } else {
        return theme.palette.errorColor
    }
}

export interface ViewTaskProps {
    currTaskId: Identifier | String | null
    goBack: () => void
    electionEventRecord?: Sequent_Backend_Election_Event
    isModal?: boolean
}

export const ViewTask: React.FC<ViewTaskProps> = ({
    currTaskId,
    goBack,
    electionEventRecord,
    isModal = false,
}) => {
    const {t} = useTranslation()
    const [progressExpanded, setProgressExpanded] = useState(true)
    const {globalSettings} = useContext(SettingsContext)
    const [exportDocumentId, setExportDocumentId] = useState<string | undefined>(undefined)
    const [downloading, setDownloading] = useState<boolean>(false)

    const {showTasksBackButton} = useTasksPermissions()

    const {data: taskData} = useQuery<GetTaskByIdQuery>(GET_TASK_BY_ID, {
        variables: {task_id: currTaskId},
        skip: !currTaskId,
        pollInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
    })

    const task = taskData?.sequent_backend_tasks_execution[0]

    if (!task) {
        return <CircularProgress />
    }

    const Content = (
        <>
            <Accordion
                sx={{width: "100%"}}
                expanded={progressExpanded}
                onChange={() => setProgressExpanded(!progressExpanded)}
            >
                <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                    <WizardStyles.AccordionTitle>
                        {t("tasksScreen.taskInformation")}
                    </WizardStyles.AccordionTitle>
                    <WizardStyles.CeremonyStatus
                        sx={{
                            backgroundColor: statusColor(
                                task?.execution_status as ETaskExecutionStatus
                            ),
                            color: theme.palette.background.default,
                        }}
                        label={t("tasksScreen.status", {
                            status: task?.execution_status as ETaskExecutionStatus,
                        })}
                    />
                </AccordionSummary>
                <WizardStyles.AccordionDetails>
                    <TableContainer component={Paper}>
                        <Table aria-label="task details table">
                            <TableBody>
                                <TableRow>
                                    <TableCell sx={{fontWeight: "500", width: "40%"}}>
                                        {t("tasksScreen.column.type")}
                                    </TableCell>
                                    <TableCell>
                                        {task?.type && t(`tasksScreen.tasksExecution.${task.type}`)}
                                    </TableCell>
                                </TableRow>
                                <TableRow>
                                    <TableCell align="left" sx={{fontWeight: "500"}}>
                                        {t("tasksScreen.column.executed_by_user")}
                                    </TableCell>
                                    <TableCell align="left">{task?.executed_by_user}</TableCell>
                                </TableRow>
                                <TableRow>
                                    <TableCell align="left" sx={{fontWeight: "500"}}>
                                        {t("tasksScreen.column.start_at")}
                                    </TableCell>
                                    <TableCell align="left">
                                        {task?.start_at && new Date(task.start_at).toLocaleString()}
                                    </TableCell>
                                </TableRow>
                                <TableRow>
                                    <TableCell align="left" sx={{fontWeight: "500"}}>
                                        {t("tasksScreen.column.end_at")}
                                    </TableCell>
                                    <TableCell align="left">
                                        {task?.end_at && new Date(task.end_at).toLocaleString()}
                                    </TableCell>
                                </TableRow>
                            </TableBody>
                        </Table>
                    </TableContainer>
                </WizardStyles.AccordionDetails>
            </Accordion>

            <Logs logs={task?.logs} />
        </>
    )

    if (isModal) {
        return (
            <Dialog
                open={true}
                variant="info"
                handleClose={goBack}
                title={t("tasksScreen.taskInformation")}
                ok={t("tasksScreen.ok")}
                fullWidth={true}
                maxWidth="md"
            >
                {Content}
            </Dialog>
        )
    }

    return (
        <WizardStyles.WizardContainer>
            <WizardStyles.ContentWrapper>
                <WizardStyles.ContentBox>{Content}</WizardStyles.ContentBox>
            </WizardStyles.ContentWrapper>

            <WizardStyles.FooterContainer>
                <WizardStyles.StyledFooter>
                    {showTasksBackButton ? (
                        <CancelButton className="list-actions" onClick={goBack}>
                            <ArrowBackIosIcon />
                            {t("common.label.back")}
                        </CancelButton>
                    ) : null}
                    {task?.election_event_id && task?.annotations?.document_id ? (
                        <DownloaButton
                            onClick={() => {
                                setDownloading(true)
                                setExportDocumentId(task?.annotations?.document_id)
                            }}
                            disabled={
                                downloading ||
                                task?.execution_status !== ETaskExecutionStatus.SUCCESS
                            }
                            label={t("tasksScreen.widget.downloadDocument")}
                        >
                            <DownloadIcon />
                        </DownloaButton>
                    ) : null}
                </WizardStyles.StyledFooter>

                {exportDocumentId && (
                    <>
                        <DownloadDocument
                            documentId={exportDocumentId ?? ""}
                            electionEventId={task?.election_event_id ?? ""}
                            fileName={null}
                            onDownload={() => {
                                setDownloading(false)
                                setExportDocumentId(undefined)
                            }}
                        />
                    </>
                )}
            </WizardStyles.FooterContainer>
        </WizardStyles.WizardContainer>
    )
}
