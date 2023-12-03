// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    Sequent_Backend_Election_Event,
    Sequent_Backend_Keys_Ceremony,
} from "@/gql/graphql"

import React, {useState} from "react"
import {
    Toolbar,
} from "react-admin"
import { useTranslation } from "react-i18next"

import { Dialog } from "@sequentech/ui-essentials"
import {
    IKeysCeremonyExecutionStatus as EStatus,
    IKeysCeremonyTrusteeStatus as TStatus,
    IExecutionStatus
} from "@/services/KeyCeremony"

import CloseIcon from "@mui/icons-material/Close"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import CheckIcon from '@mui/icons-material/Check'
import HourglassEmptyIcon from '@mui/icons-material/HourglassEmpty'
import Button from "@mui/material/Button"
import {styled} from "@mui/material/styles"
import {Box} from "@mui/material"
import Table from '@mui/material/Table'
import TableBody from '@mui/material/TableBody'
import TableCell from '@mui/material/TableCell'
import TableContainer from '@mui/material/TableContainer'
import TableHead from '@mui/material/TableHead'
import TableRow from '@mui/material/TableRow'
import Paper from '@mui/material/Paper'

const CancelButton = styled(Button)`
    margin-left: auto;
`

const StyledToolbar = styled(Toolbar)`
    flex-direction: row;
    justify-content: space-between;
`

const BackButton = styled(Button)`
    margin-right: auto;
    background-color: ${({theme}) => theme.palette.grey[100]};
    color:  ${({theme}) => theme.palette.brandColor};
`

const StyledBox = styled(Box)`
    margin-top: 30px;
    margin-bottom: 30px;
`

const DoneIcon = styled(CheckIcon)`
    color: ${({theme}) => theme.palette.brandSuccess};
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
                <TableContainer component={Paper}>
                    <Table sx={{ minWidth: 650 }} aria-label="simple table">
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
                                    sx={{ '&:last-child td, &:last-child th': { border: 0 } }}
                                >
                                    <TableCell component="th" scope="row">
                                        {trustee.name}
                                    </TableCell>
                                    <TableCell align="center">
                                        {trustee.status != TStatus.WAITING
                                            ? <HourglassEmptyIcon />
                                            : <DoneIcon />
                                        }
                                    </TableCell>
                                    <TableCell align="center">
                                        {(
                                            trustee.status == TStatus.WAITING ||
                                            trustee.status == TStatus.KEY_GENERATED
                                        )
                                            ? <HourglassEmptyIcon />
                                            : <DoneIcon />
                                        }
                                    </TableCell>
                                    <TableCell align="center">
                                        {(
                                            trustee.status == TStatus.WAITING ||
                                            trustee.status == TStatus.KEY_GENERATED ||
                                            trustee.status == TStatus.KEY_RETRIEVED
                                        )
                                            ? <HourglassEmptyIcon />
                                            : <DoneIcon />
                                        }
                                    </TableCell>
                                </TableRow>
                            )) ?? null}
                        </TableBody>
                    </Table>
                </TableContainer>
            </StyledBox>
            <StyledToolbar>
                <BackButton
                    color="info"
                    onClick={goBack}
                >
                    <ArrowBackIosIcon />
                    {t("common.label.back")}
                </BackButton>
                {cancellable() 
                    ? <CancelButton
                        onClick={() => setOpenConfirmationModal(true)}
                    >
                        <CloseIcon />
                        {t("keysGeneration.ceremonyStep.cancel")}
                    </CancelButton>
                    : null
                }
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
