// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Sequent_Backend_Election_Event, Sequent_Backend_Keys_Ceremony} from "@/gql/graphql"

import React, {useContext, useMemo, useState} from "react"
import {useTranslation} from "react-i18next"

import {theme, Dialog} from "@sequentech/ui-essentials"
import {
    IKeysCeremonyExecutionStatus as EStatus,
    IKeysCeremonyTrusteeStatus as TStatus,
    IExecutionStatus,
} from "@/services/KeyCeremony"
import {WizardStyles} from "@/components/styles/WizardStyles"

import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import ArrowForwardIosIcon from "@mui/icons-material/ArrowForwardIos"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import HourglassEmptyIcon from "@mui/icons-material/HourglassEmpty"
import {Accordion, AccordionSummary, Typography} from "@mui/material"
import Table from "@mui/material/Table"
import TableBody from "@mui/material/TableBody"
import TableCell from "@mui/material/TableCell"
import TableContainer from "@mui/material/TableContainer"
import TableHead from "@mui/material/TableHead"
import TableRow from "@mui/material/TableRow"
import Paper from "@mui/material/Paper"
import {useGetOne} from "react-admin"
import {Logs} from "../Logs"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {CancelButton} from "@/resources/Tally/styles"
import {EElectionEventCeremoniesPolicy} from "@sequentech/ui-core"

export const statusColor: (status: EStatus) => string = (status) => {
    if (status === EStatus.USER_CONFIGURATION) {
        return theme.palette.warning.light
    } else if (status === EStatus.STARTED) {
        return theme.palette.warning.light
    } else if (status === EStatus.IN_PROGRESS) {
        return theme.palette.info.main
    } else if (status === EStatus.SUCCESS) {
        return theme.palette.brandSuccess
    } else if (status === EStatus.CANCELLED) {
        return theme.palette.errorColor
    } else {
        return theme.palette.errorColor
    }
}

export interface CeremonyStepProps {
    message?: React.ReactNode
    currentCeremonyId: string
    setCurrentCeremony?: (keysCeremony: Sequent_Backend_Keys_Ceremony) => void
    electionEvent: Sequent_Backend_Election_Event
    goNext?: () => void
    isNextDisabled?: boolean
    goBack: () => void
}

export const CeremonyStep: React.FC<CeremonyStepProps> = ({
    message,
    currentCeremonyId,
    setCurrentCeremony,
    electionEvent,
    goBack,
    goNext,
    isNextDisabled = false,
}) => {
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const [openConfirmationModal, setOpenConfirmationModal] = useState(false)
    const [progressExpanded, setProgressExpanded] = useState(true)

    const {data: ceremony} = useGetOne<Sequent_Backend_Keys_Ceremony>(
        "sequent_backend_keys_ceremony",
        {
            id: currentCeremonyId ?? null,
        },
        {
            refetchInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
            onSuccess: (data) => {
                setCurrentCeremony && setCurrentCeremony(data)
            },
        }
    )
    const orderedTrustees = useMemo(
        () => [...(ceremony?.status?.trustees ?? [])].sort((a, b) => a.name.localeCompare(b.name)),
        [ceremony?.status?.trustees]
    )

    const confirmCancelCeremony = () => {}

    const status: IExecutionStatus = ceremony?.status

    const isAutomaticCeremony =
        electionEvent.presentation?.ceremonies_policy ===
            EElectionEventCeremoniesPolicy.AUTOMATED_CEREMONIES &&
        ceremony?.settings?.policy === EElectionEventCeremoniesPolicy.AUTOMATED_CEREMONIES

    return (
        <WizardStyles.WizardContainer>
            <WizardStyles.ContentWrapper>
                {!status?.public_key && message}
                <Accordion
                    sx={{width: "100%"}}
                    expanded={progressExpanded}
                    onChange={() => setProgressExpanded(!progressExpanded)}
                >
                    <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                        <WizardStyles.AccordionTitle>
                            {t("keysGeneration.ceremonyStep.progressHeader")}
                        </WizardStyles.AccordionTitle>
                        <WizardStyles.CeremonyStatus
                            sx={{
                                backgroundColor: statusColor(
                                    (ceremony?.execution_status as EStatus) ??
                                        EStatus.USER_CONFIGURATION
                                ),
                                color: theme.palette.background.default,
                            }}
                            className="keys-ceremony-status"
                            label={t("keysGeneration.ceremonyStep.executionStatus", {
                                status: ceremony?.execution_status ?? EStatus.IN_PROGRESS,
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
                                        <TableCell>
                                            {t("keysGeneration.ceremonyStep.header.trusteeName")}
                                        </TableCell>
                                        <TableCell align="center">
                                            {t("keysGeneration.ceremonyStep.header.fragment")}
                                        </TableCell>
                                        {!isAutomaticCeremony && (
                                            <>
                                                <TableCell align="center">
                                                    {t(
                                                        "keysGeneration.ceremonyStep.header.downloaded"
                                                    )}
                                                </TableCell>
                                                <TableCell align="center">
                                                    {t(
                                                        "keysGeneration.ceremonyStep.header.checked"
                                                    )}
                                                </TableCell>
                                            </>
                                        )}
                                    </TableRow>
                                </TableHead>
                                <TableBody>
                                    {orderedTrustees?.map((trustee) => {
                                        return (
                                            <TableRow
                                                key={trustee.name as any}
                                                sx={{
                                                    "&:last-child td, &:last-child th": {border: 0},
                                                }}
                                            >
                                                <TableCell component="th" scope="row">
                                                    {trustee.name}
                                                </TableCell>
                                                <TableCell align="center">
                                                    {trustee.status === TStatus.WAITING ? (
                                                        <HourglassEmptyIcon />
                                                    ) : (
                                                        <WizardStyles.DoneIcon />
                                                    )}
                                                </TableCell>
                                                {!isAutomaticCeremony && (
                                                    <>
                                                        <TableCell align="center">
                                                            {trustee.status === TStatus.WAITING ||
                                                            trustee.status ===
                                                                TStatus.KEY_GENERATED ? (
                                                                <HourglassEmptyIcon />
                                                            ) : (
                                                                <WizardStyles.DoneIcon />
                                                            )}
                                                        </TableCell>
                                                        <TableCell align="center">
                                                            {trustee.status === TStatus.WAITING ||
                                                            trustee.status ===
                                                                TStatus.KEY_GENERATED ||
                                                            trustee.status ===
                                                                TStatus.KEY_RETRIEVED ? (
                                                                <HourglassEmptyIcon />
                                                            ) : (
                                                                <WizardStyles.DoneIcon />
                                                            )}
                                                        </TableCell>
                                                    </>
                                                )}
                                            </TableRow>
                                        )
                                    }) ?? null}
                                </TableBody>
                            </Table>
                        </TableContainer>
                    </WizardStyles.AccordionDetails>
                </Accordion>

                <Logs logs={status?.logs} />
            </WizardStyles.ContentWrapper>

            <WizardStyles.FooterContainer>
                <WizardStyles.StyledFooter>
                    <CancelButton onClick={goBack} className="list-actions">
                        <ArrowBackIosIcon />
                        {t("common.label.back")}
                    </CancelButton>
                    {!!goNext && !isAutomaticCeremony && (
                        <WizardStyles.NextButton
                            color="info"
                            onClick={goNext}
                            disabled={isNextDisabled && !status.public_key}
                        >
                            <ArrowForwardIosIcon />
                            {t("common.label.next")}
                        </WizardStyles.NextButton>
                    )}
                </WizardStyles.StyledFooter>
            </WizardStyles.FooterContainer>
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
        </WizardStyles.WizardContainer>
    )
}
