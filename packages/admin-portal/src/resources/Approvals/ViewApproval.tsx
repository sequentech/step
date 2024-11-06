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
import TableRow, {TableRowTypeMap} from "@mui/material/TableRow"
import Paper from "@mui/material/Paper"
import {Identifier, useGetOne} from "react-admin"
import {Logs} from "@/components/Logs"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useQuery} from "@apollo/client"
import {CancelButton} from "../Tally/styles"
// import { ETaskExecutionStatus } from '@sequentech/ui-core'
// import {EApprovalExecutionStatus} from "@sequentech/ui-core"
import {GET_TASK_BY_ID} from "@/queries/GetTaskById"
import {ListApprovalsMatches} from "./ListApprovalsMatches"
import {OverridableComponent} from "@mui/material/OverridableComponent"

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

    // const {data: taskData} = useQuery(GET_TASK_BY_ID, {
    //     variables: {task_id: currApprovalId},
    //     skip: !currApprovalId,
    //     pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
    // })

    const {
        data: task,
        isLoading,
        error,
        refetch,
    } = useGetOne("sequent_backend_applications", {id: currApprovalId})

    console.log("aa task:", task)
    // const task = taskData?.sequent_backend_tasks_execution[0]

    if (!task) {
        return <CircularProgress />
    }

    const renderDetails = () => {
        if (!task.applicant_data || typeof task.applicant_data !== "object") {
            return (
                <TableRow>
                    <TableCell colSpan={2}>{t("common.noData")}</TableCell>
                </TableRow>
            )
        }

        const formatValue = (value: any): string | React.ReactNode => {
            if (value === null || value === undefined) {
                return "-"
            }

            // Handle different data types
            if (value instanceof Date) {
                return value.toLocaleString()
            }
            if (typeof value === "boolean") {
                return value ? "Yes" : "No"
            }
            if (typeof value === "object") {
                return JSON.stringify(value)
            }

            return String(value)
        }

        return Object.entries(task.applicant_data).map(([key, value], index) => (
            <TableRow key={index}>
                <TableCell
                    sx={{
                        fontWeight: "500",
                        width: "40%",
                        textTransform: "capitalize",
                    }}
                >
                    {/* Try to translate the key, fallback to formatted key if no translation exists */}
                    {t(`applicantData.${key}`, {
                        defaultValue: key.replace(/_/g, " "),
                    })}
                </TableCell>
                <TableCell>{formatValue(value)}</TableCell>
            </TableRow>
        ))
    }

    const Content = (
        <>
            <Accordion
                sx={{width: "100%"}}
                expanded={progressExpanded}
                // onChange={() => setProgressExpanded(!progressExpanded)}
            >
                <AccordionSummary expandIcon={false}>
                    <WizardStyles.AccordionTitle>
                        {t("approvalsScreen.approvalInformation")}
                    </WizardStyles.AccordionTitle>
                </AccordionSummary>
                <WizardStyles.AccordionDetails sx={{marginBottom: "3rem"}}>
                    <TableContainer component={Paper}>
                        <Table aria-label="approvals details table">
                            <TableBody>{renderDetails()}</TableBody>
                        </Table>
                    </TableContainer>
                </WizardStyles.AccordionDetails>
            </Accordion>
            {/* <Logs logs={task?.logs} /> */}
            <ListApprovalsMatches
                electionEventId={electionEventId}
                electionId={electionId}
                task={task}
                goBack={goBack}
            />
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
