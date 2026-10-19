// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useQuery} from "@apollo/client/react"
import {
    Alert,
    Box,
    Button,
    Chip,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    MenuItem,
    Paper,
    Stack,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    TextField,
    Typography,
} from "@mui/material"
import {useTranslation} from "react-i18next"
import type {IBallotLogPage, IBallotLogRow} from "@sequentech/ui-core"
import {BALLOT_BOX_LOG, BALLOT_BOX_OWN} from "../queries/BallotBoxLog"
import {normalizeBallotLogPrefix} from "../services/BallotLogSearch"
import {BallotBoxEntry} from "./BallotBoxEntry"

export interface BallotBoxLogViewProps {
    page?: IBallotLogPage
    ownBallots: IBallotLogRow[]
    loading: boolean
    error: boolean
    search: string
    onSearch: (value: string) => void
    limit: number
    onLimit: (limit: number) => void
    canPrevious: boolean
    onPrevious: () => void
    onNext: () => void
    onRetry: () => void
    onOpen: (ballot: IBallotLogRow) => void
    formatTime: (timestamp: string) => string
}

export const BallotBoxLogView: React.FC<BallotBoxLogViewProps> = ({
    page,
    ownBallots,
    loading,
    error,
    search,
    onSearch,
    limit,
    onLimit,
    canPrevious,
    onPrevious,
    onNext,
    onRetry,
    onOpen,
    formatTime,
}) => {
    const {t} = useTranslation()
    const [help, setHelp] = useState(false)
    const [copyStatus, setCopyStatus] = useState<"copied" | "copyFailed">()
    const prefix = normalizeBallotLogPrefix(search)
    const ownIds = new Set(ownBallots.map((ballot) => ballot.ballot_id))
    const channel = (value: string) =>
        t(`ballotPrivacy.channels.${value.toLowerCase()}`, {defaultValue: value})
    const copy = async (id: string) => {
        try {
            await navigator.clipboard.writeText(id)
            setCopyStatus("copied")
        } catch {
            setCopyStatus("copyFailed")
        }
    }
    const code = (value: string) => {
        if (!prefix || !value.replace(/-/g, "").startsWith(prefix)) return value
        const length = prefix.length + (prefix.length > 4 ? 1 : 0)
        return (
            <>
                <Box component="mark" sx={{bgcolor: "warning.light"}}>
                    {value.slice(0, length)}
                </Box>
                {value.slice(length)}
            </>
        )
    }
    return (
        <Stack spacing={2} sx={{minWidth: 0}}>
            <Box>
                <Typography variant="h5" component="h2">
                    {t("ballotPrivacy.title")}
                </Typography>
                <Typography>{t("ballotPrivacy.subtitle")}</Typography>
                <Stack direction="row" useFlexGap flexWrap="wrap" gap={1} sx={{mt: 2}}>
                    {["private", "signed", "device"].map((key) => (
                        <Chip key={key} size="small" label={t(`ballotPrivacy.${key}`)} />
                    ))}
                </Stack>
            </Box>
            {ownBallots.map((ballot) => (
                <Alert
                    key={ballot.ballot_id}
                    severity="success"
                    sx={{"& .MuiAlert-message": {minWidth: 0, flex: 1}}}
                >
                    <Typography fontWeight={700}>{t("ballotPrivacy.own")}</Typography>
                    <Typography component="div" variant="h6" sx={{overflowWrap: "anywhere"}}>
                        {ballot.ballot_id}
                    </Typography>
                    <Typography variant="body2">
                        {formatTime(ballot.cast_at)} · {channel(ballot.voting_channel)}
                    </Typography>
                    <Typography component="div" variant="body2">
                        {t("ballotPrivacy.listedAs")} <Chip label={ballot.pseudonym} size="small" />
                    </Typography>
                    <Stack direction="row" useFlexGap flexWrap="wrap" gap={1} sx={{mt: 1}}>
                        <Button variant="contained" onClick={() => onOpen(ballot)}>
                            {t("ballotPrivacy.check")}
                        </Button>
                        <Button onClick={() => void copy(ballot.ballot_id)}>
                            {t("ballotPrivacy.copy")}
                        </Button>
                    </Stack>
                </Alert>
            ))}
            {copyStatus && (
                <Typography role="status" variant="body2">
                    {t(`ballotPrivacy.${copyStatus}`)}
                </Typography>
            )}
            <Stack direction={{xs: "column", sm: "row"}} gap={1} alignItems={{sm: "center"}}>
                <TextField
                    fullWidth
                    label={t("ballotPrivacy.search")}
                    placeholder={t("ballotPrivacy.placeholder")}
                    value={search}
                    onChange={(event) => onSearch(event.target.value)}
                    error={prefix === null}
                    helperText={prefix === null ? t("ballotPrivacy.invalidPrefix") : undefined}
                    slotProps={{htmlInput: {autoCapitalize: "characters", spellCheck: false}}}
                />
                {search && <Button onClick={() => onSearch("")}>{t("ballotPrivacy.clear")}</Button>}
            </Stack>
            {error ? (
                <Alert
                    severity="error"
                    action={<Button onClick={onRetry}>{t("ballotPrivacy.retry")}</Button>}
                >
                    {t("ballotPrivacy.loadError")}
                </Alert>
            ) : null}
            {loading ? (
                <CircularProgress size={24} aria-label={t("ballotPrivacy.checking")} />
            ) : null}
            {page && prefix !== null && !loading && !error ? (
                <>
                    <Typography variant="body2" role="status">
                        {prefix
                            ? t(
                                  page.match_count_capped
                                      ? "ballotPrivacy.capped"
                                      : "ballotPrivacy.matches",
                                  {count: page.match_count, total: page.total}
                              )
                            : t("ballotPrivacy.ballots", {count: page.total})}
                    </Typography>
                    {!page.ballots.length ? (
                        <Alert severity="info">
                            {prefix
                                ? t("ballotPrivacy.noMatch", {prefix: search})
                                : t("ballotPrivacy.noBallots")}
                        </Alert>
                    ) : (
                        <Paper variant="outlined" sx={{overflow: "hidden"}}>
                            <Table
                                sx={{tableLayout: "fixed", display: {xs: "none", sm: "table"}}}
                                aria-label={t("ballotPrivacy.title")}
                            >
                                <TableHead>
                                    <TableRow>
                                        {["ballotId", "pseudonym", "castAt", "channel"].map(
                                            (key) => (
                                                <TableCell key={key}>
                                                    {t(`ballotPrivacy.${key}`)}
                                                </TableCell>
                                            )
                                        )}
                                    </TableRow>
                                </TableHead>
                                <TableBody>
                                    {page.ballots.map((ballot) => (
                                        <TableRow key={ballot.ballot_id}>
                                            <TableCell>
                                                <Button
                                                    onClick={() => onOpen(ballot)}
                                                    sx={{fontWeight: 700, p: 0}}
                                                >
                                                    {code(ballot.ballot_id)}
                                                </Button>
                                                {ownIds.has(ballot.ballot_id) && (
                                                    <Typography variant="caption" display="block">
                                                        {t("ballotPrivacy.yours")}
                                                    </Typography>
                                                )}
                                            </TableCell>
                                            <TableCell>
                                                <Chip size="small" label={code(ballot.pseudonym)} />
                                            </TableCell>
                                            <TableCell sx={{overflowWrap: "anywhere"}}>
                                                {formatTime(ballot.cast_at)}
                                            </TableCell>
                                            <TableCell>{channel(ballot.voting_channel)}</TableCell>
                                        </TableRow>
                                    ))}
                                </TableBody>
                            </Table>
                            <Box
                                component="ul"
                                sx={{
                                    display: {xs: "block", sm: "none"},
                                    listStyle: "none",
                                    m: 0,
                                    p: 0,
                                }}
                            >
                                {page.ballots.map((ballot) => (
                                    <Box
                                        component="li"
                                        key={ballot.ballot_id}
                                        sx={{p: 1.5, borderBottom: 1, borderColor: "divider"}}
                                    >
                                        <Button
                                            onClick={() => onOpen(ballot)}
                                            sx={{p: 0, fontWeight: 700}}
                                        >
                                            {code(ballot.ballot_id)}
                                        </Button>
                                        {ownIds.has(ballot.ballot_id) && (
                                            <Chip
                                                size="small"
                                                label={t("ballotPrivacy.yours")}
                                                sx={{ml: 1}}
                                            />
                                        )}
                                        <Stack direction="row" useFlexGap flexWrap="wrap" gap={1}>
                                            <Chip size="small" label={code(ballot.pseudonym)} />
                                            <Typography variant="caption">
                                                {formatTime(ballot.cast_at)} ·{" "}
                                                {channel(ballot.voting_channel)}
                                            </Typography>
                                        </Stack>
                                    </Box>
                                ))}
                            </Box>
                        </Paper>
                    )}
                    <Stack direction="row" useFlexGap flexWrap="wrap" gap={1} alignItems="center">
                        <TextField
                            select
                            size="small"
                            label={t("ballotPrivacy.rows")}
                            value={limit}
                            onChange={(event) => onLimit(Number(event.target.value))}
                            sx={{minWidth: 140}}
                        >
                            {[10, 25, 50].map((size) => (
                                <MenuItem key={size} value={size}>
                                    {size}
                                </MenuItem>
                            ))}
                        </TextField>
                        <Button disabled={!canPrevious} onClick={onPrevious}>
                            {t("ballotPrivacy.previous")}
                        </Button>
                        <Button disabled={!page.next_cursor} onClick={onNext}>
                            {t("ballotPrivacy.next")}
                        </Button>
                    </Stack>
                </>
            ) : null}
            <Button sx={{alignSelf: "flex-start"}} onClick={() => setHelp(true)}>
                {t("ballotPrivacy.help")}
            </Button>
            <Dialog
                open={help}
                onClose={() => setHelp(false)}
                fullWidth
                maxWidth="sm"
                aria-labelledby="ballot-log-help"
            >
                <DialogTitle id="ballot-log-help">{t("ballotPrivacy.help")}</DialogTitle>
                <DialogContent>{t("ballotPrivacy.helpText")}</DialogContent>
                <DialogActions>
                    <Button onClick={() => setHelp(false)}>{t("ballotPrivacy.close")}</Button>
                </DialogActions>
            </Dialog>
        </Stack>
    )
}

export const BallotBoxLog: React.FC<{
    electionEventId: string
    electionId?: string
    formatTime: (timestamp: string) => string
}> = ({electionEventId, electionId, formatTime}) => {
    const [search, setSearch] = useState("")
    const [prefix, setPrefix] = useState<string | null>("")
    const [limit, setLimit] = useState(10)
    const [cursors, setCursors] = useState<Array<string | null>>([null])
    const [selected, setSelected] = useState<IBallotLogRow>()
    useEffect(() => {
        const timer = setTimeout(() => {
            setPrefix(normalizeBallotLogPrefix(search))
            setCursors([null])
        }, 300)
        return () => clearTimeout(timer)
    }, [search])
    useEffect(() => {
        setSearch("")
        setPrefix("")
        setCursors([null])
        setSelected(undefined)
    }, [electionEventId, electionId])
    const list = useQuery<{ballot_box_log: {payload: IBallotLogPage}}>(BALLOT_BOX_LOG, {
        variables: {
            electionEventId,
            electionId,
            prefix,
            cursor: cursors[cursors.length - 1],
            limit,
        },
        skip: prefix === null,
        fetchPolicy: "network-only",
    })
    const own = useQuery<{ballot_box_own: {payload: {ballots: IBallotLogRow[]}}}>(BALLOT_BOX_OWN, {
        variables: {electionEventId, electionId},
        fetchPolicy: "no-cache",
    })
    const page = list.data?.ballot_box_log.payload
    return (
        <>
            <BallotBoxLogView
                page={page}
                ownBallots={own.data?.ballot_box_own.payload.ballots ?? []}
                loading={list.loading || normalizeBallotLogPrefix(search) !== prefix}
                error={!!list.error}
                search={search}
                onSearch={setSearch}
                limit={limit}
                onLimit={(value) => {
                    setLimit(value)
                    setCursors([null])
                }}
                canPrevious={cursors.length > 1}
                onPrevious={() => setCursors((values) => values.slice(0, -1))}
                onNext={() => {
                    if (page?.next_cursor) setCursors((values) => [...values, page.next_cursor])
                }}
                onRetry={() => {
                    void list.refetch().catch(() => undefined)
                    void own.refetch().catch(() => undefined)
                }}
                onOpen={setSelected}
                formatTime={formatTime}
            />
            {selected && (
                <BallotBoxEntry
                    electionEventId={electionEventId}
                    electionId={selected.election_id}
                    ballotId={selected.ballot_id}
                    onClose={() => setSelected(undefined)}
                />
            )}
        </>
    )
}
