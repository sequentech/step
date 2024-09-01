// SPDX-License-Identifier: AGPL-3.0-only

import {Sequent_Backend_Election_Event, Sequent_Backend_Tasks_Execution} from "@/gql/graphql"

import React, {useState} from "react"
import {useTranslation} from "react-i18next"

import {theme, Dialog} from "@sequentech/ui-essentials"
import {IKeysCeremonyExecutionStatus as EStatus} from "@/services/KeyCeremony"
import {WizardStyles} from "@/components/styles/WizardStyles"

import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import {Accordion, AccordionSummary, Typography} from "@mui/material"
import Table from "@mui/material/Table"
import TableBody from "@mui/material/TableBody"
import TableCell from "@mui/material/TableCell"
import TableContainer from "@mui/material/TableContainer"
import TableRow from "@mui/material/TableRow"
import Paper from "@mui/material/Paper"
import {Identifier, useGetOne} from "react-admin"
import {Logs} from "@/components/Logs"
import {ETaskExecutionStatus, ITaskExecuted} from "@sequentech/ui-core"

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
    currTaskId: Identifier | null // Sequent_Backend_Tasks_Execution
    electionEventRecord: Sequent_Backend_Election_Event
    onViewList: () => void
}

export const ViewTask: React.FC<ViewTaskProps> = ({
    currTaskId,
    electionEventRecord,
    onViewList,
}) => {
    const {t} = useTranslation()
    const [progressExpanded, setProgressExpanded] = useState(true)

    const {data: task} = useGetOne<Sequent_Backend_Tasks_Execution>(
        "sequent_backend_tasks_execution",
        {
            id: currTaskId,
        }
    )

    return (
        <>
            <WizardStyles.ContentBox>
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
                            <Table sx={{minWidth: 650}} aria-label="task details table">
                                <TableBody>
                                    <TableRow>
                                        <TableCell sx={{fontWeight:"500"}}>{t("tasksScreen.column.type")}</TableCell>
                                        <TableCell>{task?.type}</TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell align="left" sx={{fontWeight:"500"}}>
                                            {t("tasksScreen.column.executed_by_user")}
                                        </TableCell>
                                        <TableCell align="left">{task?.executed_by_user}</TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell align="left" sx={{fontWeight:"500"}}>
                                            {t("tasksScreen.column.start_at")}
                                        </TableCell>
                                        <TableCell align="left">
                                            {task && new Date(task.start_at).toLocaleString()}
                                        </TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell align="left" sx={{fontWeight:"500"}}>
                                            {t("tasksScreen.column.end_at")}
                                        </TableCell>
                                        <TableCell align="left">
                                            {task && new Date(task.end_at).toLocaleString()}
                                        </TableCell>
                                    </TableRow>
                                </TableBody>
                            </Table>
                        </TableContainer>
                    </WizardStyles.AccordionDetails>
                </Accordion>

                <Logs logs={task?.logs} />
            </WizardStyles.ContentBox>
            <WizardStyles.Toolbar>
                <WizardStyles.BackButton color="info" onClick={onViewList}>
                    <ArrowBackIosIcon />
                    {t("common.label.back")}
                </WizardStyles.BackButton>
            </WizardStyles.Toolbar>
        </>
    )
}
