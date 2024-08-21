// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
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
import TableHead from "@mui/material/TableHead"
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
    currTaskId: Identifier | null //Sequent_Backend_Tasks_Execution
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
                        {/* <WizardStyles.AccordionTitle>
                            {t("keysGeneration.ceremonyStep.progressHeader")}
                        </WizardStyles.AccordionTitle> */}
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
                        <Typography variant="body2">
                            {t("keysGeneration.ceremonyStep.description")}
                        </Typography>
                        <TableContainer component={Paper}>
                            <Table sx={{minWidth: 650}} aria-label="simple table">
                                <TableHead>
                                    <TableRow>
                                        <TableCell>{t("tasksScreen.column.type")}</TableCell>
                                        <TableCell align="center">
                                            {t("tasksScreen.column.executed_by_user_id")}
                                        </TableCell>
                                        <TableCell align="center">
                                            {t("tasksScreen.column.start_at")}
                                        </TableCell>
                                        <TableCell align="center">
                                            {t("tasksScreen.column.end_at")}
                                        </TableCell>
                                    </TableRow>
                                </TableHead>
                                <TableBody>
                                    {task && (
                                        <TableRow
                                            key={task?.name as any}
                                            sx={{
                                                "&:last-child td, &:last-child th": {border: 0},
                                            }}
                                        >
                                            <TableCell component="th" scope="row">
                                                {task.type}
                                            </TableCell>
                                            <TableCell align="center">
                                                {task.executed_by_user_id}
                                            </TableCell>
                                            <TableCell align="center">{task.start_at}</TableCell>
                                            <TableCell align="center">{task.end_at}</TableCell>
                                        </TableRow>
                                    )}
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
