// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState, type ReactNode} from "react"
import {useQuery} from "@apollo/client"
import {
    Alert,
    Box,
    Button,
    CircularProgress,
    Stack,
    Table,
    TableBody,
    TableCell,
    TableContainer,
    TableHead,
    TableRow,
    Typography,
} from "@mui/material"
import LockOutlinedIcon from "@mui/icons-material/LockOutlined"
import {Dialog} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"
import type {IForensicAddressDetail} from "@sequentech/ui-core"
import {FORENSIC_NETWORK_DETAIL} from "@/queries/ForensicNetwork"
import {IPermissions} from "@/types/keycloak"
import {AdminDateTime} from "@/components/AdminDateTime"
import {ForensicPseudonym, ForensicCountry, ForensicBallotCount} from "./ForensicFields"

export const ForensicAddressDialog: React.FC<{
    electionEventId: string
    electionId?: string
    electionLabel?: ReactNode
    shared?: boolean
    ip: string
    onClose: () => void
}> = ({electionEventId, electionId, electionLabel, shared = false, ip, onClose}) => {
    const {t} = useTranslation()
    const [cursors, setCursors] = useState<Array<string | null>>([null])
    const {data, loading, error, refetch} = useQuery<{
        forensic_network_detail: {payload: IForensicAddressDetail}
    }>(FORENSIC_NETWORK_DETAIL, {
        variables: {
            electionEventId,
            electionId,
            ip,
            cursor: cursors[cursors.length - 1],
            limit: 25,
        },
        fetchPolicy: "no-cache",
        context: {headers: {"x-hasura-role": IPermissions.FORENSIC_READ}},
    })
    const detail = data?.forensic_network_detail.payload
    return (
        <Dialog
            open
            fullWidth
            maxWidth="md"
            variant="info"
            title={t("ballotPrivacy.forensic.detailTitle", {ip})}
            ok={t("ballotPrivacy.close")}
            handleClose={onClose}
            className="forensic-address-dialog"
        >
            <Stack spacing={2} sx={{minWidth: 0}}>
                {loading ? (
                    <Box sx={{display: "flex", justifyContent: "center", py: 4}}>
                        <CircularProgress aria-label={t("common.label.loadingData")} />
                    </Box>
                ) : error || !detail ? (
                    <Alert
                        severity="error"
                        action={
                            <Button onClick={() => void refetch().catch(() => undefined)}>
                                {t("ballotPrivacy.retry")}
                            </Button>
                        }
                    >
                        {t("ballotPrivacy.forensic.error")}
                    </Alert>
                ) : (
                    <>
                        <Box
                            component="dl"
                            sx={{
                                display: "grid",
                                gridTemplateColumns: {
                                    xs: "96px minmax(0, 1fr)",
                                    sm: "96px minmax(0, 1fr) 96px minmax(0, 1fr)",
                                },
                                columnGap: 2,
                                rowGap: 1,
                                alignItems: "baseline",
                                m: 0,
                            }}
                        >
                            {[
                                [
                                    t("ballotPrivacy.forensic.address"),
                                    <Box
                                        component="span"
                                        sx={{fontFamily: "monospace", overflowWrap: "anywhere"}}
                                    >
                                        {detail.ip}
                                    </Box>,
                                ],
                                [
                                    t("dashboard.ipAddress.country"),
                                    <ForensicCountry code={detail.country} showName />,
                                ],
                                [
                                    t("ballotPrivacy.forensic.ballots"),
                                    <Box sx={{display: "inline-flex"}}>
                                        <ForensicBallotCount
                                            count={detail.vote_count}
                                            shared={shared}
                                        />
                                    </Box>,
                                ],
                                [
                                    t("dashboard.ipAddress.ElectionName"),
                                    electionLabel ?? t("ballotPrivacy.forensic.allElections"),
                                ],
                                [
                                    t("ballotPrivacy.forensic.first"),
                                    <AdminDateTime
                                        value={detail.first_ballot}
                                        event={electionEventId}
                                        seconds
                                    />,
                                ],
                                [
                                    t("ballotPrivacy.forensic.last"),
                                    <AdminDateTime
                                        value={detail.last_ballot}
                                        event={electionEventId}
                                        seconds
                                    />,
                                ],
                            ].map(([label, value], index) => (
                                <React.Fragment key={index}>
                                    <Typography
                                        component="dt"
                                        variant="caption"
                                        color="text.secondary"
                                    >
                                        {label}
                                    </Typography>
                                    <Typography
                                        component="dd"
                                        variant="body2"
                                        sx={{m: 0, overflowWrap: "anywhere"}}
                                    >
                                        {value}
                                    </Typography>
                                </React.Fragment>
                            ))}
                        </Box>
                        {detail.ballots.length ? (
                            <TableContainer
                                sx={{
                                    maxHeight: 350,
                                    border: "1px solid",
                                    borderColor: "divider",
                                    borderRadius: 1,
                                }}
                            >
                                <Table
                                    size="small"
                                    stickyHeader
                                    sx={{
                                        "tableLayout": "fixed",
                                        "& .MuiTableCell-root": {
                                            px: {xs: 1, sm: 1.5},
                                            py: 1,
                                            overflowWrap: "anywhere",
                                        },
                                        "& th": {
                                            bgcolor: "#F7F9FC",
                                            color: "#475569",
                                            fontWeight: 600,
                                        },
                                        "& tr:last-child td": {borderBottom: 0},
                                    }}
                                >
                                    <TableHead>
                                        <TableRow>
                                            <TableCell>{t("ballotPrivacy.pseudonym")}</TableCell>
                                            <TableCell>{t("ballotPrivacy.ballotId")}</TableCell>
                                            <TableCell>{t("ballotPrivacy.castAt")}</TableCell>
                                        </TableRow>
                                    </TableHead>
                                    <TableBody>
                                        {detail.ballots.map((ballot) => (
                                            <TableRow key={`${ballot.ballot_id}-${ballot.cast_at}`}>
                                                <TableCell>
                                                    <ForensicPseudonym code={ballot.pseudonym} />
                                                </TableCell>
                                                <TableCell
                                                    sx={{
                                                        fontFamily: "monospace",
                                                        fontSize: 13,
                                                        fontWeight: 600,
                                                    }}
                                                >
                                                    {ballot.ballot_id ?? "—"}
                                                </TableCell>
                                                <TableCell sx={{fontSize: 12}}>
                                                    <AdminDateTime
                                                        value={ballot.cast_at}
                                                        event={electionEventId}
                                                        seconds
                                                    />
                                                </TableCell>
                                            </TableRow>
                                        ))}
                                    </TableBody>
                                </Table>
                            </TableContainer>
                        ) : (
                            <Typography variant="body2" color="text.secondary" sx={{py: 3}}>
                                {t("ballotPrivacy.forensic.empty")}
                            </Typography>
                        )}
                        {cursors.length > 1 || detail.next_cursor ? (
                            <Stack direction="row" justifyContent="flex-end" spacing={1}>
                                <Button
                                    disabled={cursors.length === 1}
                                    onClick={() => setCursors((values) => values.slice(0, -1))}
                                >
                                    {t("ballotPrivacy.previous")}
                                </Button>
                                <Button
                                    disabled={!detail.next_cursor}
                                    onClick={() => {
                                        if (detail.next_cursor)
                                            setCursors((values) => [...values, detail.next_cursor])
                                    }}
                                >
                                    {t("ballotPrivacy.next")}
                                </Button>
                            </Stack>
                        ) : null}
                        <Alert
                            icon={<LockOutlinedIcon fontSize="small" />}
                            severity="info"
                            variant="outlined"
                            sx={{
                                "bgcolor": "#F7F9FC",
                                "borderColor": "#DFE6EF",
                                "color": "text.primary",
                                "fontSize": 12,
                                "& .MuiAlert-icon": {color: "#64748B"},
                            }}
                        >
                            {t("ballotPrivacy.forensic.detailHelp")}
                        </Alert>
                    </>
                )}
            </Stack>
        </Dialog>
    )
}
