// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import React, {useContext, useState} from "react"
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
import {Identifier} from "react-admin"
import {Logs} from "@/components/Logs"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useQuery} from "@apollo/client"
import {CancelButton} from "../Tally/styles"
// import { ETaskExecutionStatus } from '@sequentech/ui-core'
// import {EApprovalExecutionStatus} from "@sequentech/ui-core"
import {GET_TASK_BY_ID} from "@/queries/GetTaskById"
import { ListApprovalsMatches } from './ListApprovalsMatches'

// export const statusColor: (status: string) => string = (status) => {
//     if (status === EApprovalExecutionStatus.STARTED) {
//         return theme.palette.warning.light
//     } else if (status === EApprovalExecutionStatus.IN_PROGRESS) {
//         return theme.palette.info.main
//     } else if (status === EApprovalExecutionStatus.SUCCESS) {
//         return theme.palette.brandSuccess
//     } else if (status === EApprovalExecutionStatus.CANCELLED) {
//         return theme.palette.errorColor
//     } else {
//         return theme.palette.errorColor
//     }
// }

export interface ViewApprovalProps {
    electionEventId: string
    electionId?: string
    currApprovalId: Identifier | String | null
    goBack: () => void
    electionEventRecord?: Sequent_Backend_Election_Event
    isModal?: boolean
}

export const ViewApproval: React.FC<ViewApprovalProps> = ({
    electionEventId,
    electionId,
    currApprovalId,
    goBack,
    electionEventRecord,
    isModal = false,
}) => {
    const {t} = useTranslation()
    const [progressExpanded, setProgressExpanded] = useState(true)
    const {globalSettings} = useContext(SettingsContext)

    const {data: taskData} = useQuery(GET_TASK_BY_ID, {
        variables: {task_id: currApprovalId},
        skip: !currApprovalId,
        pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
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
                        {t("approvalsScreen.approvalInformation")}
                    </WizardStyles.AccordionTitle>
                    {/* <WizardStyles.CeremonyStatus
                        sx={{
                            backgroundColor: statusColor(
                                task?.execution_status as EApprovalExecutionStatus
                            ),
                            color: theme.palette.background.default,
                        }}
                        label={t("approvalsScreen.status", {
                            status: task?.execution_status as EApprovalExecutionStatus,
                        })}
                    /> */}
                </AccordionSummary>
                <WizardStyles.AccordionDetails>
                    <TableContainer component={Paper}>
                        <Table aria-label="approvals details table">
                            <TableBody>
                                <TableRow>
                                    <TableCell sx={{fontWeight: "500", width: "40%"}}>
                                        {t("approvalsScreen.column.type")}
                                    </TableCell>
                                    <TableCell>
                                        {task?.type &&
                                            t(`approvalsScreen.approvalsExecution.${task.type}`)}
                                    </TableCell>
                                </TableRow>
                                <TableRow>
                                    <TableCell align="left" sx={{fontWeight: "500"}}>
                                        {t("approvalsScreen.column.executed_by_user")}
                                    </TableCell>
                                    <TableCell align="left">{task?.executed_by_user}</TableCell>
                                </TableRow>
                                <TableRow>
                                    <TableCell align="left" sx={{fontWeight: "500"}}>
                                        {t("approvalsScreen.column.start_at")}
                                    </TableCell>
                                    <TableCell align="left">
                                        {task?.start_at && new Date(task.start_at).toLocaleString()}
                                    </TableCell>
                                </TableRow>
                                <TableRow>
                                    <TableCell align="left" sx={{fontWeight: "500"}}>
                                        {t("approvalsScreen.column.end_at")}
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
            {/* <Logs logs={task?.logs} /> */}
            <ListApprovalsMatches electionEventId={electionEventId} electionId={electionId} />
        </>
    )

    if (isModal) {
        return (
            <Dialog
                open={true}
                variant="info"
                handleClose={goBack}
                title={t("approvalsScreen.taskInformation")}
                ok={t("approvalsScreen.ok")}
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
                    <CancelButton className="list-actions" onClick={goBack}>
                        <ArrowBackIosIcon />
                        {t("common.label.back")}
                    </CancelButton>
                </WizardStyles.StyledFooter>
            </WizardStyles.FooterContainer>
        </WizardStyles.WizardContainer>
    )
}
