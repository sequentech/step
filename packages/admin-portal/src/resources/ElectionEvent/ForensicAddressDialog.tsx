// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {useQuery} from "@apollo/client"
import {
    Alert,
    Button,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    Stack,
    Typography,
    useMediaQuery,
    useTheme,
} from "@mui/material"
import {useTranslation} from "react-i18next"
import type {IForensicAddressDetail} from "@sequentech/ui-core"
import {FORENSIC_NETWORK_DETAIL} from "@/queries/ForensicNetwork"
import {IPermissions} from "@/types/keycloak"
import {AdminDateTime} from "@/components/AdminDateTime"

export const ForensicAddressDialog: React.FC<{
    electionEventId: string
    electionId?: string
    ip: string
    onClose: () => void
}> = ({electionEventId, electionId, ip, onClose}) => {
    const {t} = useTranslation()
    const theme = useTheme()
    const narrow = useMediaQuery(theme.breakpoints.down("sm"))
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
            maxWidth="sm"
            fullScreen={narrow}
            onClose={onClose}
            aria-labelledby="forensic-address-title"
        >
            <DialogTitle id="forensic-address-title" sx={{overflowWrap: "anywhere"}}>
                {t("ballotPrivacy.forensic.detailTitle", {ip})}
            </DialogTitle>
            <DialogContent>
                <Stack spacing={2}>
                    <Alert severity="info">{t("ballotPrivacy.forensic.detailHelp")}</Alert>
                    {loading ? (
                        <CircularProgress />
                    ) : error || !detail ? (
                        <Alert severity="error">{t("ballotPrivacy.forensic.error")}</Alert>
                    ) : (
                        <>
                            <Typography>
                                {detail.country ?? "—"} ·{" "}
                                {t("ballotPrivacy.ballots", {count: detail.vote_count})}
                            </Typography>
                            <Typography variant="body2">
                                {t("ballotPrivacy.forensic.first")}:{" "}
                                <AdminDateTime
                                    value={detail.first_ballot}
                                    event={electionEventId}
                                    seconds
                                />
                            </Typography>
                            <Typography variant="body2">
                                {t("ballotPrivacy.forensic.last")}:{" "}
                                <AdminDateTime
                                    value={detail.last_ballot}
                                    event={electionEventId}
                                    seconds
                                />
                            </Typography>
                            {detail.ballots.length ? (
                                <Stack component="ul" sx={{m: 0, pl: 2}} spacing={1}>
                                    {detail.ballots.map((ballot) => (
                                        <Typography
                                            component="li"
                                            key={`${ballot.ballot_id}-${ballot.cast_at}`}
                                            variant="body2"
                                            sx={{overflowWrap: "anywhere"}}
                                        >
                                            {ballot.pseudonym} · {ballot.ballot_id ?? "—"}
                                            <br />
                                            <AdminDateTime
                                                value={ballot.cast_at}
                                                event={electionEventId}
                                                seconds
                                            />
                                        </Typography>
                                    ))}
                                </Stack>
                            ) : (
                                <Typography>{t("ballotPrivacy.forensic.empty")}</Typography>
                            )}
                            <Stack direction="row">
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
                        </>
                    )}
                </Stack>
            </DialogContent>
            <DialogActions>
                <Button onClick={() => void refetch().catch(() => undefined)} disabled={loading}>
                    {t("ballotPrivacy.retry")}
                </Button>
                <Button onClick={onClose}>{t("ballotPrivacy.close")}</Button>
            </DialogActions>
        </Dialog>
    )
}
