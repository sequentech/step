// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Sequent_Backend_Election_Event, Sequent_Backend_Keys_Ceremony} from "@/gql/graphql"

import React, {useState} from "react"
import {Toolbar} from "react-admin"
import {useTranslation} from "react-i18next"

import { theme, Dialog } from "@sequentech/ui-essentials"
import {
    IKeysCeremonyExecutionStatus as EStatus,
    IKeysCeremonyTrusteeStatus as TStatus,
    IExecutionStatus,
} from "@/services/KeyCeremony"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"

import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import CloseIcon from "@mui/icons-material/Close"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import DoneOutlineIcon from "@mui/icons-material/DoneOutline"
import HourglassEmptyIcon from "@mui/icons-material/HourglassEmpty"
import Button from "@mui/material/Button"
import {styled} from "@mui/material/styles"
import {Accordion, AccordionDetails, AccordionSummary, Box, Chip, Typography} from "@mui/material"
import Table from "@mui/material/Table"
import TableBody from "@mui/material/TableBody"
import TableCell from "@mui/material/TableCell"
import TableContainer from "@mui/material/TableContainer"
import TableHead from "@mui/material/TableHead"
import TableRow from "@mui/material/TableRow"
import Paper from "@mui/material/Paper"

export const statusColor: (status: string) => string = (status) => {
    if (status == EStatus.NOT_STARTED) {
        return theme.palette.warning.light
    } else if (status == EStatus.IN_PROCESS) {
        return theme.palette.info.main
    } else if (status == EStatus.SUCCESS) {
        return theme.palette.brandSuccess
    } else if (status == EStatus.CANCELLED) {
        return theme.palette.errorColor
    } else {
        return theme.palette.errorColor
    }
}

const CancelButton = styled(Button)`
    margin-left: auto;
    background-color: ${({theme}) => theme.palette.grey[100]};
    color: ${({theme}) => theme.palette.errorColor};
    border-color: ${({theme}) => theme.palette.errorColor};

    &:hover {
        background-color: ${({theme}) => theme.palette.errorColor};
    }
`

const StyledToolbar = styled(Toolbar)`
    flex-direction: row;
    justify-content: space-between;
`

const BackButton = styled(Button)`
    margin-right: auto;
    background-color: ${({theme}) => theme.palette.grey[100]};
    color: ${({theme}) => theme.palette.brandColor};
`

const StyledBox = styled(Box)`
    margin-top: 30px;
    margin-bottom: 30px;
`

const DoneIcon = styled(DoneOutlineIcon)`
    color: ${({theme}) => theme.palette.brandSuccess};
`

const AccordionTitle = styled(ElectionHeaderStyles.Title)`
    margin-bottom: 0 !important;
`

const AccordionDetails2 = styled(AccordionDetails)`
    padding-top: 0;
    margin-top: -10px;
`

const CeremonyStatus = styled(Chip)`
    margin-top: 6px;
    margin-left: auto;
    margin-right: 10px;
    font-weight: bold;
`

export interface CeremonyStepProps {
    currentCeremony: Sequent_Backend_Keys_Ceremony | null
    electionEvent: Sequent_Backend_Election_Event
    goBack: () => void
}

export const CeremonyStep: React.FC<CeremonyStepProps> = ({
    currentCeremony,
    electionEvent,
    goBack,
}) => {
    console.log(`ceremony step with currentCeremony.id=${currentCeremony?.id ?? null}`)
    const {t} = useTranslation()
    const [openConfirmationModal, setOpenConfirmationModal] = useState(false)
    const [progressExpanded, setProgressExpanded] = useState(true)
    const [logsExpanded, setLogsExpanded] = useState(true)

    const confirmCancelCeremony = () => {}
    const cancellable = () => {
        return (
            currentCeremony?.execution_status == EStatus.NOT_STARTED ||
            currentCeremony?.execution_status == EStatus.IN_PROCESS
        )
    }
    const status: IExecutionStatus = currentCeremony?.status

    return (
        <>
            <StyledBox>
                <Accordion
                    sx={{width: "100%"}}
                    expanded={progressExpanded}
                    onChange={() => setProgressExpanded(!progressExpanded)}
                >
                    <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                        <AccordionTitle>
                            {t("keysGeneration.ceremonyStep.progressHeader")}
                        </AccordionTitle>
                        <CeremonyStatus

                            sx={{
                                backgroundColor: statusColor(
                                    currentCeremony?.execution_status
                                        ?? EStatus.NOT_STARTED
                                ),
                                color: theme.palette.background.default,
                            }}
                            label={t(
                                "keysGeneration.ceremonyStep.executionStatus",
                                {status: electionEvent.public_key
                                    ? EStatus.IN_PROCESS
                                    : currentCeremony?.execution_status,
                            })}
                        />
                    </AccordionSummary>
                    <AccordionDetails2>
                        <Typography variant="body2">
                            {t("keysGeneration.ceremonyStep.description")}
                        </Typography>
                        <TableContainer component={Paper}>
                            <Table sx={{minWidth: 650}} aria-label="simple table">
                                <TableHead>
                                    <TableRow>
                                        <TableCell>
                                            {t("keysGeneration.ceremonyStep.header.trusteeName")}
                                        </TableCell>
                                        <TableCell align="center">
                                            {t("keysGeneration.ceremonyStep.header.fragment")}
                                        </TableCell>
                                        <TableCell align="center">
                                            {t("keysGeneration.ceremonyStep.header.downloaded")}
                                        </TableCell>
                                        <TableCell align="center">
                                            {t("keysGeneration.ceremonyStep.header.checked")}
                                        </TableCell>
                                    </TableRow>
                                </TableHead>
                                <TableBody>
                                    {status.trustees.map((trustee) => (
                                        <TableRow
                                            key={trustee.name as any}
                                            sx={{"&:last-child td, &:last-child th": {border: 0}}}
                                        >
                                            <TableCell component="th" scope="row">
                                                {trustee.name}
                                            </TableCell>
                                            <TableCell align="center">
                                                {!electionEvent.public_key ? (
                                                    <HourglassEmptyIcon />
                                                ) : (
                                                    <DoneIcon />
                                                )}
                                            </TableCell>
                                            <TableCell align="center">
                                                {trustee.status == TStatus.WAITING ||
                                                trustee.status == TStatus.KEY_GENERATED ? (
                                                    <HourglassEmptyIcon />
                                                ) : (
                                                    <DoneIcon />
                                                )}
                                            </TableCell>
                                            <TableCell align="center">
                                                {trustee.status == TStatus.WAITING ||
                                                trustee.status == TStatus.KEY_GENERATED ||
                                                trustee.status == TStatus.KEY_RETRIEVED ? (
                                                    <HourglassEmptyIcon />
                                                ) : (
                                                    <DoneIcon />
                                                )}
                                            </TableCell>
                                        </TableRow>
                                    )) ?? null}
                                </TableBody>
                            </Table>
                        </TableContainer>
                    </AccordionDetails2>
                </Accordion>

                <Accordion
                    sx={{width: "100%"}}
                    expanded={logsExpanded}
                    onChange={() => setLogsExpanded(!logsExpanded)}
                >
                    <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                        <AccordionTitle>
                            {t("keysGeneration.ceremonyStep.logsHeader.title")}
                        </AccordionTitle>
                    </AccordionSummary>
                    <AccordionDetails2>
                        {status?.logs.length > 0 ? (
                            <Paper sx={{width: "100%", overflow: "hidden"}}>
                                <TableContainer>
                                    ?{" "}
                                    <Table sx={{maxHeight: 450}} aria-label="simple table">
                                        <TableHead>
                                            <TableRow>
                                                <TableCell>
                                                    {t(
                                                        "keysGeneration.ceremonyStep.logsHeader.date"
                                                    )}
                                                </TableCell>
                                                <TableCell align="left">
                                                    {t(
                                                        "keysGeneration.ceremonyStep.logsHeader.entry"
                                                    )}
                                                </TableCell>
                                            </TableRow>
                                        </TableHead>
                                        <TableBody>
                                            {status.logs.map((log) => (
                                                <TableRow
                                                    key={log?.created_date as any}
                                                    sx={{
                                                        "&:last-child td, &:last-child th": {
                                                            border: 0,
                                                        },
                                                    }}
                                                >
                                                    <TableCell component="th" scope="row">
                                                        {log?.created_date}
                                                    </TableCell>
                                                    <TableCell align="left">
                                                        {log?.log_text}
                                                    </TableCell>
                                                </TableRow>
                                            )) ?? null}
                                        </TableBody>
                                    </Table>
                                </TableContainer>
                            </Paper>
                        ) : (
                            <Box>
                                <Typography variant="body2">
                                    {t("keysGeneration.ceremonyStep.emptyLogs")}
                                </Typography>
                            </Box>
                        )}
                    </AccordionDetails2>
                </Accordion>
            </StyledBox>
            <StyledToolbar>
                <BackButton color="info" onClick={goBack}>
                    <ArrowBackIosIcon />
                    {t("common.label.back")}
                </BackButton>
                {cancellable() ? (
                    <CancelButton onClick={() => setOpenConfirmationModal(true)}>
                        <CloseIcon />
                        {t("keysGeneration.ceremonyStep.cancel")}
                    </CancelButton>
                ) : null}
            </StyledToolbar>
            <Dialog
                variant="warning"
                open={openConfirmationModal}
                ok={t("keysGeneration.ceremonyStep.confirmdDialog.ok")}
                cancel={t("keysGeneration.ceremonyStep.confirmdDialog.cancel")}
                title={t("keysGeneration.ceremonyStep.confirmdDialog.title")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmCancelCeremony()
                    }
                    setOpenConfirmationModal(false)
                }}
            >
                {t("keysGeneration.ceremonyStep.confirmdDialog.description")}
            </Dialog>
        </>
    )
}
