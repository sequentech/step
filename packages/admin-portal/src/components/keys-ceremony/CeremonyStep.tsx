// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Sequent_Backend_Election_Event, Sequent_Backend_Keys_Ceremony} from "@/gql/graphql"

import React, {useContext, useEffect, useMemo, useState} from "react"
import {useTranslation} from "react-i18next"
import {gql, useQuery} from "@apollo/client"

import {theme, Dialog} from "@sequentech/ui-essentials"
import {
    hasGeneratedCeremonyKeys,
    IKeysCeremonyExecutionStatus as EStatus,
    IKeysCeremonyTrusteeStatus as TStatus,
    IExecutionStatus,
} from "@/services/KeyCeremony"
import {WizardStyles} from "@/components/styles/WizardStyles"

import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import ArrowForwardIosIcon from "@mui/icons-material/ArrowForwardIos"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline"
import HourglassEmptyIcon from "@mui/icons-material/HourglassEmpty"
import {Accordion, AccordionSummary, Alert, Box, Typography} from "@mui/material"
import {getContrastRatio} from "@mui/material/styles"
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
import {CopyableFingerprint} from "@/components/CopyableFingerprint"
import {colonHex, hex, sha256} from "@/lib/signing/der"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useKeysPermissions} from "@/resources/ElectionEvent/useKeysPermissions"
import {IPermissions} from "@/types/keycloak"

/** A key ceremony's DKG board, with the Configuration it serves. */
export const KEYS_CEREMONY_CONFIGURATION = gql`
    query KeysCeremonyConfiguration($tenantId: uuid!, $keysCeremonyId: uuid!) {
        sequent_backend_protocol_board(
            where: {
                tenant_id: {_eq: $tenantId}
                keys_ceremony_id: {_eq: $keysCeremonyId}
                parent_id: {_is_null: true}
            }
        ) {
            id
            name
            public_manager_message
        }
    }
`

interface IKeysCeremonyConfiguration {
    sequent_backend_protocol_board: Array<{
        id: string
        name: string
        public_manager_message: string | null
    }>
}

/** The length of the sender's name, then the sender's Ed25519 key. */
const NAME_LENGTH_BYTES = 8
const KEY_BYTES = 32

/**
 * The fingerprint of the protocol manager key that a stored Configuration
 * names, in the form each trustee server logs it when it accepts that
 * Configuration: SHA-256 of the key's 32 bytes, every byte as two hexadecimal
 * digits, separated by colons.
 *
 * Hasura gives the stored message as `\x` and hexadecimal digits. The message
 * starts with its sender: the name, as a big-endian 64-bit length and that
 * many bytes, then the sender's 32-byte key. A trustee accepts a
 * Configuration only if its sender's key is the protocol manager key that it
 * names.
 */
export const managerKeyFingerprint = async (
    publicManagerMessage: string,
    subtle?: SubtleCrypto
): Promise<string> => {
    if (!/^\\x(?:[0-9a-fA-F]{2})+$/.test(publicManagerMessage)) {
        throw new Error("Invalid public configuration")
    }
    const bytes = new Uint8Array(
        (publicManagerMessage.slice(2).match(/../g) ?? []).map((pair) => parseInt(pair, 16))
    )
    if (bytes.length < NAME_LENGTH_BYTES + KEY_BYTES) {
        throw new Error("Invalid public configuration")
    }
    const view = new DataView(bytes.buffer)
    const keyStart = NAME_LENGTH_BYTES + view.getUint32(4)
    if (view.getUint32(0) !== 0 || keyStart + KEY_BYTES > bytes.length) {
        throw new Error("Invalid public configuration")
    }
    return colonHex(hex(await sha256(bytes.slice(keyStart, keyStart + KEY_BYTES), subtle)))
}

/**
 * The protocol manager key of a key ceremony, for the election board to
 * compare with what each trustee server logs.
 */
const ManagerKeyFingerprint: React.FC<{keysCeremonyId: string}> = ({keysCeremonyId}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const {data, error} = useQuery<IKeysCeremonyConfiguration>(KEYS_CEREMONY_CONFIGURATION, {
        variables: {tenantId, keysCeremonyId},
        skip: !tenantId || !keysCeremonyId,
        context: {headers: {"x-hasura-role": IPermissions.ADMIN_CEREMONY}},
    })
    const boards = data?.sequent_backend_protocol_board ?? []
    const board = boards.length === 1 ? boards[0] : undefined
    const message = board?.public_manager_message ?? null
    const [fingerprint, setFingerprint] = useState<string | null>(null)
    const [failed, setFailed] = useState(false)

    useEffect(() => {
        let current = true
        setFingerprint(null)
        setFailed(false)
        if (message) {
            managerKeyFingerprint(message).then(
                (value) => current && setFingerprint(value),
                () => current && setFailed(true)
            )
        }
        return () => {
            current = false
        }
    }, [message])

    if (error || failed || boards.length > 1 || (board && !message)) {
        return (
            <Alert severity="warning" className="keys-ceremony-manager-key-error">
                {t("keysGeneration.ceremonyStep.managerKey.readError")}
            </Alert>
        )
    }
    if (!board || !fingerprint) {
        return null
    }
    return (
        <Box className="keys-ceremony-manager-key" sx={{width: "100%"}}>
            <Typography variant="subtitle2" component="div">
                {t("keysGeneration.ceremonyStep.managerKey.title")}
            </Typography>
            <Box sx={{fontFamily: "monospace", fontSize: "0.85rem", wordBreak: "break-all"}}>
                <CopyableFingerprint value={fingerprint} short={fingerprint} />
            </Box>
            <Typography variant="caption" component="div">
                {t("keysGeneration.ceremonyStep.managerKey.help", {board: board.name})}
            </Typography>
        </Box>
    )
}

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

const trusteeStepIcon = (status: TStatus, pending: Array<TStatus>, label: string) => {
    if (status === TStatus.HALTED) {
        return <WizardStyles.HaltedIcon titleAccess={label} />
    }
    return pending.includes(status) ? (
        <HourglassEmptyIcon titleAccess={label} />
    ) : (
        <WizardStyles.DoneIcon titleAccess={label} />
    )
}

export interface CeremonyStepProps {
    message?: React.ReactNode
    currentCeremonyId: string
    setCurrentCeremony?: (keysCeremony: Sequent_Backend_Keys_Ceremony) => void
    electionEvent: Sequent_Backend_Election_Event
    goNext?: () => void
    isNextDisabled?: boolean
    goBack: () => void
    confirmCustody?: () => void
}

export const CeremonyStep: React.FC<CeremonyStepProps> = ({
    message,
    currentCeremonyId,
    setCurrentCeremony,
    electionEvent,
    goBack,
    goNext,
    isNextDisabled = false,
    confirmCustody,
}) => {
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const {canAdminCeremony} = useKeysPermissions()
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

    const stepIcon = (trusteeStatus: TStatus, pending: TStatus[]) =>
        trusteeStepIcon(
            trusteeStatus,
            pending,
            String(
                t(
                    trusteeStatus === TStatus.HALTED
                        ? "keysGeneration.ceremonyStep.halted"
                        : pending.includes(trusteeStatus)
                          ? "keysGeneration.ceremonyStep.waiting"
                          : "keysGeneration.ceremonyStep.complete"
                )
            )
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
                {(!status || !hasGeneratedCeremonyKeys(status)) && message}
                {status?.failure && <Alert severity="error">{status.failure}</Alert>}
                {canAdminCeremony && currentCeremonyId && (
                    <ManagerKeyFingerprint keysCeremonyId={currentCeremonyId} />
                )}
                <Accordion
                    sx={{width: "100%"}}
                    expanded={progressExpanded}
                    onChange={() => setProgressExpanded(!progressExpanded)}
                >
                    <AccordionSummary
                        id={`${currentCeremonyId}-progress-heading`}
                        aria-controls={`${currentCeremonyId}-progress-content`}
                        expandIcon={<ExpandMoreIcon />}
                    >
                        <WizardStyles.AccordionTitle>
                            {t("keysGeneration.ceremonyStep.progressHeader")}
                        </WizardStyles.AccordionTitle>
                        <WizardStyles.CeremonyStatus
                            sx={{
                                backgroundColor: statusColor(
                                    (ceremony?.execution_status as EStatus) ??
                                        EStatus.USER_CONFIGURATION
                                ),
                                color:
                                    getContrastRatio(
                                        statusColor(
                                            (ceremony?.execution_status as EStatus) ??
                                                EStatus.USER_CONFIGURATION
                                        ),
                                        theme.palette.common.white
                                    ) >= 4.5
                                        ? theme.palette.common.white
                                        : theme.palette.common.black,
                            }}
                            className="keys-ceremony-status"
                            label={String(
                                t("keysGeneration.ceremonyStep.executionStatus", {
                                    status: ceremony?.execution_status ?? EStatus.IN_PROGRESS,
                                })
                            )}
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
                                                key={trustee.name}
                                                sx={{
                                                    "&:last-child td, &:last-child th": {border: 0},
                                                }}
                                            >
                                                <TableCell component="th" scope="row">
                                                    {trustee.name}
                                                </TableCell>
                                                <TableCell align="center">
                                                    {stepIcon(trustee.status, [
                                                        TStatus.WAITING,
                                                        TStatus.SHARES_POSTED,
                                                    ])}
                                                </TableCell>
                                                {!isAutomaticCeremony && (
                                                    <>
                                                        <TableCell align="center">
                                                            {stepIcon(trustee.status, [
                                                                TStatus.WAITING,
                                                                TStatus.SHARES_POSTED,
                                                                TStatus.KEY_GENERATED,
                                                            ])}
                                                        </TableCell>
                                                        <TableCell align="center">
                                                            {stepIcon(trustee.status, [
                                                                TStatus.WAITING,
                                                                TStatus.SHARES_POSTED,
                                                                TStatus.KEY_GENERATED,
                                                                TStatus.CUSTODY_REQUEST_RETRIEVED,
                                                            ])}
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

                <Logs logs={status?.logs} event={electionEvent} />
            </WizardStyles.ContentWrapper>

            <WizardStyles.FooterContainer>
                <WizardStyles.StyledFooter>
                    <CancelButton onClick={goBack} className="list-actions">
                        <ArrowBackIosIcon />
                        {t("common.label.back")}
                    </CancelButton>
                    {!!confirmCustody && (
                        <CancelButton onClick={confirmCustody} className="list-actions">
                            <CheckCircleOutlineIcon />
                            {t("keysGeneration.checkStep.verifyButton")}
                        </CancelButton>
                    )}
                    {!!goNext && !isAutomaticCeremony && (
                        <WizardStyles.NextButton
                            color="info"
                            onClick={goNext}
                            disabled={isNextDisabled}
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
                ok={String(t("keysGeneration.ceremonyStep.confirmdDialog.ok"))}
                cancel={String(t("keysGeneration.ceremonyStep.confirmdDialog.cancel"))}
                title={String(t("keysGeneration.ceremonyStep.confirmdDialog.title"))}
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
