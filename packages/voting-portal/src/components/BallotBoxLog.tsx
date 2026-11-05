// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useQuery} from "@apollo/client/react"
import {
    Alert,
    Box,
    Button,
    ButtonBase,
    Chip,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    IconButton,
    InputAdornment,
    MenuItem,
    Paper,
    Stack,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    TextField,
    Tooltip,
    Typography,
} from "@mui/material"
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome"
import {
    faChevronLeft,
    faChevronRight,
    faCircleCheck,
    faCircleQuestion,
    faCopy,
    faDisplay,
    faFileSignature,
    faGlobe,
    faPhone,
    faSearch,
    faShieldHalved,
    faUserSecret,
} from "@fortawesome/free-solid-svg-icons"
import {useTranslation} from "react-i18next"
import type {Policy} from "@vitrine/verify"
import {
    pseudonymDisplay,
    stringToHtml,
    type IBallotLogPage,
    type IBallotLogRow,
} from "@sequentech/ui-core"
import {PseudonymChip} from "@sequentech/ui-essentials"
import {BALLOT_BOX_LOG, BALLOT_BOX_OWN} from "../queries/BallotBoxLog"
import {useBucketMatches} from "../hooks/useShortIdMatches"
import {
    matchedCharacters,
    parseBallotLogSearch,
    searchesAnything,
} from "../services/BallotLogSearch"
import {groupBallotId, shortBallotId} from "../services/vitrine/ballotId"
import {isPolicyIdTyped} from "../services/vitrine/policyId"
import {
    EShortIdKind,
    EShortIdStatus,
    IShortIdMatch,
    IShortIdResult,
    shortIdToLook,
} from "../services/vitrine/shortId"
import {ShortIdMatches, ShortIdServerNote} from "./BallotLocator/ShortIdMatches"
import {BallotBoxEntry} from "./BallotBoxEntry"

export interface BallotBoxLogViewProps {
    page?: IBallotLogPage
    ownBallots: IBallotLogRow[]
    loading: boolean
    error: boolean
    search: string
    onSearch: (value: string) => void
    /** The search is the election's policy ID, which no ballot has. */
    policyIdTyped?: boolean
    limit: number
    onLimit: (limit: number) => void
    canPrevious: boolean
    onPrevious: () => void
    onNext: () => void
    onRetry: () => void
    onOpen: (ballot: IBallotLogRow) => void
    formatTime: (timestamp: string) => string
    timeZoneLabel?: string
    /** The ballots that share the short ID searched for, listed from a bucket
     * of the record's servers, which are not told which ballot it is. They
     * take the place of the page. */
    shortIds?: {
        result: IShortIdResult
        onOpen: (match: IShortIdMatch) => void
        onNarrow: (typed: string) => void
    }
    /** The search was answered by the election's server, which saw the ID,
     * because no server of the record lists buckets. */
    shortIdServerSaw?: boolean
}

export const BallotBoxLogView: React.FC<BallotBoxLogViewProps> = ({
    page,
    ownBallots,
    loading,
    error,
    search,
    onSearch,
    policyIdTyped = false,
    limit,
    onLimit,
    canPrevious,
    onPrevious,
    onNext,
    onRetry,
    onOpen,
    formatTime,
    timeZoneLabel,
    shortIds,
    shortIdServerSaw = false,
}) => {
    const {t} = useTranslation()
    const [help, setHelp] = useState(false)
    const [ballotHelp, setBallotHelp] = useState(false)
    const [copyStatus, setCopyStatus] = useState<"copied" | "copyFailed">()
    const parsed = parseBallotLogSearch(search)
    const searching = !!parsed && parsed.query !== ""
    const unreadable = !policyIdTyped && (parsed === null || !searchesAnything(parsed))
    const ownIds = new Set(ownBallots.map((ballot) => ballot.tracking_id))
    const channel = (value: string) =>
        t(`ballotPrivacy.channels.${value.toLowerCase()}`, {defaultValue: value})
    const channelIcon = (value: string) =>
        value.toLowerCase() === "telephone"
            ? faPhone
            : value.toLowerCase() === "kiosk"
              ? faDisplay
              : faGlobe
    const copy = async (id: string) => {
        try {
            await navigator.clipboard.writeText(id)
            setCopyStatus("copied")
        } catch {
            setCopyStatus("copyFailed")
        }
    }
    const stageLabel = (ballot: IBallotLogRow) => t(`ballotPrivacy.stages.${ballot.stage}.label`)
    const stageTag = (ballot: IBallotLogRow) => (
        <Chip
            className={`ballot-box-log-stage ballot-box-log-stage-${ballot.stage}`}
            size="small"
            label={stageLabel(ballot)}
            sx={{
                height: 22,
                bgcolor: "#F1F5F9",
                color: "#334155",
                border: "1px solid #CBD5E1",
                fontSize: 12,
                fontWeight: 600,
            }}
        />
    )
    const code = (value: string, start: string | null | undefined) => {
        const matched = matchedCharacters(value, start ?? null)
        if (!matched) return value
        // Separators sit after every fourth character of the code.
        const length = matched + Math.floor((matched - 1) / 4)
        return (
            <>
                <Box
                    className="ballot-box-log-match"
                    component="mark"
                    sx={{bgcolor: "#FEF08A", color: "inherit", borderRadius: "2px"}}
                >
                    {value.slice(0, length)}
                </Box>
                {value.slice(length)}
            </>
        )
    }
    const yours = (
        <Chip
            className="ballot-box-log-yours"
            size="small"
            label={t("ballotPrivacy.yours")}
            sx={{
                height: 20,
                bgcolor: "#0F054C",
                color: "#FFFFFF",
                fontSize: 11,
                fontWeight: 600,
            }}
        />
    )
    const validPage = page && !unreadable && !loading && !error && !shortIds
    return (
        <Stack
            className="ballot-box-log"
            spacing={3}
            sx={{
                minWidth: 0,
                width: "100%",
                pt: 3,
                fontFamily: "Roboto, Helvetica, Arial, sans-serif",
            }}
        >
            <Box className="ballot-box-log-heading">
                <Stack
                    className="ballot-box-log-heading-row"
                    direction="row"
                    alignItems="center"
                    gap={1}
                >
                    <Typography
                        className="ballot-box-log-title"
                        variant="h5"
                        component="h2"
                        sx={{m: 0, fontWeight: 700}}
                    >
                        {t("ballotPrivacy.title")}
                    </Typography>
                    <Tooltip
                        className="ballot-box-log-help-tooltip"
                        title={t("ballotPrivacy.help")}
                    >
                        <IconButton
                            className="ballot-box-log-help-button"
                            size="small"
                            aria-label={t("ballotPrivacy.help")}
                            onClick={() => setHelp(true)}
                        >
                            <FontAwesomeIcon
                                className="ballot-box-log-help-icon"
                                icon={faCircleQuestion}
                                style={{fontSize: 18}}
                            />
                        </IconButton>
                    </Tooltip>
                </Stack>
                <Typography
                    className="ballot-box-log-subtitle"
                    color="text.secondary"
                    sx={{m: 0, mt: 1}}
                >
                    {t("ballotPrivacy.subtitle")}
                </Typography>
                <Stack
                    className="ballot-box-log-badges"
                    direction="row"
                    useFlexGap
                    flexWrap="wrap"
                    gap={1}
                    sx={{mt: 2.5}}
                >
                    {[
                        {key: "private", icon: faUserSecret},
                        {key: "signed", icon: faFileSignature},
                        {key: "device", icon: faShieldHalved},
                    ].map(({key, icon}) => (
                        <Chip
                            className="ballot-box-log-badge"
                            key={key}
                            icon={
                                <FontAwesomeIcon
                                    className="ballot-box-log-badge-icon"
                                    icon={icon}
                                />
                            }
                            label={t(`ballotPrivacy.${key}`)}
                            sx={{
                                "height": "auto",
                                "minHeight": 32,
                                "py": 0.25,
                                "bgcolor": "#F7F9FC",
                                "border": "1px solid #E2E8F0",
                                "fontSize": 13,
                                "color": "#0F054C",
                                "& .MuiChip-label": {whiteSpace: "normal"},
                                "& .MuiChip-icon": {fontSize: 14, color: "#0F054C", ml: 1.5},
                            }}
                        />
                    ))}
                </Stack>
            </Box>
            {ownBallots.map((ballot) => (
                <Box
                    className="ballot-box-log-own-ballot"
                    key={ballot.tracking_id}
                    sx={{
                        display: "flex",
                        alignItems: "center",
                        flexWrap: {xs: "wrap", md: "nowrap"},
                        gap: 2.5,
                        p: {xs: 2, md: 3},
                        border: "1px solid #059669",
                        borderRadius: "8px",
                        bgcolor: "#ECFDF5",
                        color: "#065F46",
                    }}
                >
                    <Box
                        className="ballot-box-log-own-status-icon-container"
                        sx={{
                            width: 48,
                            height: 48,
                            borderRadius: "50%",
                            bgcolor: "white",
                            border: "1px solid #A7D9CC",
                            display: {xs: "none", sm: "grid"},
                            placeItems: "center",
                            flexShrink: 0,
                        }}
                    >
                        <FontAwesomeIcon
                            className="ballot-box-log-own-status-icon"
                            icon={faCircleCheck}
                            style={{fontSize: 24, color: "#047857"}}
                        />
                    </Box>
                    <Box className="ballot-box-log-own-content" sx={{flex: 1, minWidth: 0}}>
                        <Typography
                            className="ballot-box-log-own-title"
                            sx={{
                                m: 0,
                                fontSize: 12,
                                fontWeight: 700,
                                letterSpacing: 1,
                                textTransform: "uppercase",
                            }}
                        >
                            {t("ballotPrivacy.own")}
                        </Typography>
                        <Stack
                            className="ballot-box-log-own-id-row"
                            direction="row"
                            alignItems="center"
                            useFlexGap
                            flexWrap="wrap"
                            gap={0.5}
                        >
                            <Typography
                                className="ballot-box-log-own-id"
                                component="div"
                                sx={{
                                    m: 0,
                                    fontSize: {xs: 25, sm: 30},
                                    fontWeight: 700,
                                    letterSpacing: 1.5,
                                    overflowWrap: "anywhere",
                                }}
                            >
                                {shortBallotId(ballot.tracking_id)}
                            </Typography>
                            <Tooltip
                                className="ballot-box-log-copy-tooltip"
                                title={t("ballotPrivacy.copy")}
                            >
                                <IconButton
                                    className="ballot-box-log-copy-button"
                                    size="small"
                                    aria-label={t("ballotPrivacy.copy")}
                                    onClick={() => void copy(groupBallotId(ballot.tracking_id))}
                                >
                                    <FontAwesomeIcon
                                        className="ballot-box-log-copy-icon"
                                        icon={faCopy}
                                        style={{fontSize: 16}}
                                    />
                                </IconButton>
                            </Tooltip>
                            <Tooltip
                                className="ballot-box-log-ballot-id-help-tooltip"
                                title={t("ballotPrivacy.ballotIdHelp")}
                            >
                                <IconButton
                                    className="ballot-box-log-ballot-id-help-button"
                                    size="small"
                                    aria-label={t("ballotPrivacy.ballotIdHelp")}
                                    onClick={() => setBallotHelp(true)}
                                >
                                    <FontAwesomeIcon
                                        className="ballot-box-log-ballot-id-help-icon"
                                        icon={faCircleQuestion}
                                        style={{fontSize: 16}}
                                    />
                                </IconButton>
                            </Tooltip>
                        </Stack>
                        <Stack
                            className="ballot-box-log-own-metadata"
                            direction="row"
                            useFlexGap
                            flexWrap="wrap"
                            gap={1.5}
                            alignItems="center"
                            sx={{mt: 0.5}}
                        >
                            <Typography
                                className="ballot-box-log-own-details"
                                variant="body2"
                                sx={{m: 0}}
                            >
                                {t("ballotPrivacy.stageTime", {
                                    stage: stageLabel(ballot),
                                    time: formatTime(ballot.last_step_at),
                                })}
                                {ballot.voting_channel
                                    ? ` · ${channel(ballot.voting_channel)}`
                                    : ""}
                            </Typography>
                            <Stack
                                className="ballot-box-log-own-pseudonym-label"
                                direction="row"
                                alignItems="center"
                                gap={1}
                            >
                                <Typography
                                    className="ballot-box-log-own-pseudonym-text"
                                    variant="body2"
                                    sx={{m: 0}}
                                >
                                    {t("ballotPrivacy.listedAs")}
                                </Typography>
                                <PseudonymChip className="ballot-box-log-own-pseudonym">
                                    {pseudonymDisplay(ballot.pseudonym)}
                                </PseudonymChip>
                            </Stack>
                        </Stack>
                    </Box>
                    <Button
                        className="ballot-box-log-own-check-button"
                        variant="contained"
                        startIcon={
                            <FontAwesomeIcon
                                className="ballot-box-log-own-check-icon"
                                icon={faShieldHalved}
                            />
                        }
                        onClick={() => onOpen(ballot)}
                        sx={{
                            flexShrink: 0,
                            width: {xs: "100%", md: "auto"},
                            textTransform: "none",
                            px: 3,
                        }}
                    >
                        {t("ballotPrivacy.check")}
                    </Button>
                </Box>
            ))}
            {copyStatus && (
                <Typography
                    className="ballot-box-log-copy-status"
                    role="status"
                    variant="body2"
                    sx={{m: 0}}
                >
                    {t(`ballotPrivacy.${copyStatus}`)}
                </Typography>
            )}
            <Paper
                className="ballot-box-log-list"
                variant="outlined"
                sx={{
                    overflow: "hidden",
                    borderRadius: "8px",
                    borderColor: "#DFE6EF",
                    boxShadow: "none",
                }}
            >
                <Stack
                    className="ballot-box-log-search-controls"
                    direction={{xs: "column", sm: "row"}}
                    useFlexGap
                    flexWrap="wrap"
                    gap={1.5}
                    alignItems={{sm: "center"}}
                    sx={{p: 2.5}}
                >
                    <TextField
                        className="ballot-box-log-search"
                        size="small"
                        fullWidth
                        placeholder={t("ballotPrivacy.placeholder")}
                        value={search}
                        onChange={(event) => onSearch(event.target.value)}
                        error={unreadable}
                        helperText={
                            unreadable
                                ? t("ballotPrivacy.invalidPrefix")
                                : policyIdTyped
                                  ? t("ballotPrivacy.policyId")
                                  : undefined
                        }
                        slotProps={{
                            htmlInput: {
                                "aria-label": t("ballotPrivacy.search"),
                                "autoCapitalize": "characters",
                                "spellCheck": false,
                            },
                            input: {
                                startAdornment: (
                                    <InputAdornment
                                        className="ballot-box-log-search-adornment"
                                        position="start"
                                    >
                                        <FontAwesomeIcon
                                            className="ballot-box-log-search-icon"
                                            icon={faSearch}
                                        />
                                    </InputAdornment>
                                ),
                            },
                        }}
                        sx={{maxWidth: {sm: 360}}}
                    />
                    {search && (
                        <Button
                            className="ballot-box-log-clear-button"
                            onClick={() => onSearch("")}
                        >
                            {t("ballotPrivacy.clear")}
                        </Button>
                    )}
                    {validPage && (
                        <Typography
                            className="ballot-box-log-count"
                            variant="body2"
                            role="status"
                            sx={{m: 0, ml: {sm: "auto"}, color: "text.secondary"}}
                        >
                            {searching
                                ? t(
                                      page.match_count_capped
                                          ? "ballotPrivacy.capped"
                                          : "ballotPrivacy.matches",
                                      {count: page.match_count, total: page.total}
                                  )
                                : t("ballotPrivacy.ballots", {count: page.total})}
                            {searching &&
                                page.tracking_matches != null &&
                                page.pseudonym_matches != null && (
                                    <Box
                                        className="ballot-box-log-count-split"
                                        component="span"
                                        sx={{display: "block"}}
                                    >
                                        {t("ballotPrivacy.matchesBoth", {
                                            tracking: page.tracking_matches,
                                            pseudonym: page.pseudonym_matches,
                                        })}
                                    </Box>
                                )}
                        </Typography>
                    )}
                </Stack>
                {error && (
                    <Alert
                        className="ballot-box-log-error"
                        severity="error"
                        action={
                            <Button className="ballot-box-log-retry-button" onClick={onRetry}>
                                {t("ballotPrivacy.retry")}
                            </Button>
                        }
                    >
                        {t("ballotPrivacy.loadError")}
                    </Alert>
                )}
                {loading && (
                    <Box
                        className="ballot-box-log-loading-container"
                        sx={{p: 3, textAlign: "center"}}
                    >
                        <CircularProgress
                            className="ballot-box-log-loading"
                            size={24}
                            aria-label={t("ballotPrivacy.checking")}
                        />
                    </Box>
                )}
                {shortIds && !loading && !error && !unreadable && (
                    <Box className="ballot-box-log-short-ids" sx={{p: 2.5, pt: 0}}>
                        <ShortIdMatches
                            key={shortIds.result.typed}
                            result={shortIds.result}
                            onOpen={shortIds.onOpen}
                            onNarrow={shortIds.onNarrow}
                            formatTime={(seconds) =>
                                formatTime(new Date(seconds * 1000).toISOString())
                            }
                        />
                    </Box>
                )}
                {validPage && shortIdServerSaw && (
                    <Box className="ballot-box-log-short-id-note" sx={{px: 2.5}}>
                        <ShortIdServerNote />
                    </Box>
                )}
                {validPage && (
                    <>
                        {!page.ballots.length ? (
                            <Box
                                className="ballot-box-log-empty-container"
                                sx={{p: 2.5, borderTop: "1px solid #DFE6EF"}}
                            >
                                <Alert className="ballot-box-log-empty" severity="info">
                                    {searching
                                        ? t("ballotPrivacy.noMatch", {prefix: search})
                                        : t("ballotPrivacy.noBallots")}
                                    {searching && (
                                        <Typography
                                            className="ballot-box-log-no-match-help"
                                            variant="body2"
                                            sx={{m: 0, mt: 1}}
                                        >
                                            {t("ballotPrivacy.noMatchHelp")}
                                        </Typography>
                                    )}
                                </Alert>
                            </Box>
                        ) : (
                            <>
                                <Table
                                    className="ballot-box-log-table"
                                    sx={{
                                        "tableLayout": "fixed",
                                        "display": {xs: "none", min750: "table"},
                                        "@media (min-width: 750px)": {display: "table"},
                                        "& th": {
                                            bgcolor: "#F7F9FC",
                                            color: "#64748B",
                                            fontSize: 12,
                                            fontWeight: 600,
                                            textTransform: "uppercase",
                                            letterSpacing: 0.4,
                                        },
                                        "& td, & th": {px: 2.5, py: 1.5, borderColor: "#DFE6EF"},
                                    }}
                                    aria-label={t("ballotPrivacy.title")}
                                >
                                    <TableHead className="ballot-box-log-table-head">
                                        <TableRow className="ballot-box-log-table-header-row">
                                            {[
                                                ["ballotId", "22%"],
                                                ["pseudonym", "20%"],
                                                ["lastStep", "16%"],
                                                ["time", "24%"],
                                                ["channel", "18%"],
                                            ].map(([key, width]) => (
                                                <TableCell
                                                    className="ballot-box-log-table-header-cell"
                                                    key={key}
                                                    sx={{width}}
                                                >
                                                    {t(`ballotPrivacy.${key}`)}
                                                </TableCell>
                                            ))}
                                        </TableRow>
                                    </TableHead>
                                    <TableBody className="ballot-box-log-table-body">
                                        {page.ballots.map((ballot) => (
                                            <TableRow
                                                className="ballot-box-log-row"
                                                key={ballot.tracking_id}
                                                hover
                                                onClick={() => onOpen(ballot)}
                                                sx={{
                                                    cursor: "pointer",
                                                    bgcolor: ownIds.has(ballot.tracking_id)
                                                        ? "#F0FDF8"
                                                        : undefined,
                                                    borderLeft: ownIds.has(ballot.tracking_id)
                                                        ? "3px solid #34D399"
                                                        : "3px solid transparent",
                                                }}
                                            >
                                                <TableCell className="ballot-box-log-ballot-cell">
                                                    <Stack
                                                        className="ballot-box-log-ballot-id-content"
                                                        direction="row"
                                                        alignItems="center"
                                                        useFlexGap
                                                        flexWrap="wrap"
                                                        gap={1}
                                                    >
                                                        <ButtonBase
                                                            className="ballot-box-log-check-button"
                                                            onClick={(event) => {
                                                                event.stopPropagation()
                                                                onOpen(ballot)
                                                            }}
                                                            sx={{
                                                                fontWeight: 700,
                                                                fontSize: 15,
                                                                letterSpacing: 0.6,
                                                                color: "text.primary",
                                                                p: 0,
                                                                minWidth: 0,
                                                            }}
                                                        >
                                                            {code(
                                                                shortBallotId(ballot.tracking_id),
                                                                parsed?.trackingId
                                                            )}
                                                        </ButtonBase>
                                                        {ownIds.has(ballot.tracking_id) && yours}
                                                    </Stack>
                                                </TableCell>
                                                <TableCell className="ballot-box-log-pseudonym-cell">
                                                    <PseudonymChip className="ballot-box-log-pseudonym">
                                                        {code(
                                                            pseudonymDisplay(ballot.pseudonym),
                                                            parsed?.pseudonym
                                                        )}
                                                    </PseudonymChip>
                                                </TableCell>
                                                <TableCell className="ballot-box-log-stage-cell">
                                                    {stageTag(ballot)}
                                                </TableCell>
                                                <TableCell
                                                    className="ballot-box-log-timestamp-cell"
                                                    sx={{overflowWrap: "anywhere", fontSize: 14}}
                                                >
                                                    {formatTime(ballot.last_step_at)}
                                                </TableCell>
                                                <TableCell className="ballot-box-log-channel-cell">
                                                    <Stack
                                                        className="ballot-box-log-channel-content"
                                                        direction="row"
                                                        alignItems="center"
                                                        gap={1}
                                                    >
                                                        {ballot.voting_channel ? (
                                                            <>
                                                                <FontAwesomeIcon
                                                                    className="ballot-box-log-channel-icon"
                                                                    icon={channelIcon(
                                                                        ballot.voting_channel
                                                                    )}
                                                                    style={{
                                                                        color: "#64748B",
                                                                        fontSize: 12,
                                                                    }}
                                                                />
                                                                <Typography
                                                                    className="ballot-box-log-channel-label"
                                                                    variant="body2"
                                                                    sx={{m: 0}}
                                                                >
                                                                    {channel(ballot.voting_channel)}
                                                                </Typography>
                                                            </>
                                                        ) : (
                                                            <Typography
                                                                className="ballot-box-log-channel-label"
                                                                variant="body2"
                                                                color="text.secondary"
                                                                sx={{m: 0}}
                                                            >
                                                                {"—"}
                                                            </Typography>
                                                        )}
                                                        <Box
                                                            className="ballot-box-log-row-arrow-container"
                                                            sx={{ml: "auto", color: "#64748B"}}
                                                        >
                                                            <FontAwesomeIcon
                                                                className="ballot-box-log-row-arrow-icon"
                                                                icon={faChevronRight}
                                                                style={{fontSize: 12}}
                                                            />
                                                        </Box>
                                                    </Stack>
                                                </TableCell>
                                            </TableRow>
                                        ))}
                                    </TableBody>
                                </Table>
                                <Box
                                    className="ballot-box-log-mobile-list"
                                    component="ul"
                                    sx={{
                                        "display": "block",
                                        "@media (min-width: 750px)": {display: "none"},
                                        "listStyle": "none",
                                        "m": 0,
                                        "p": 0,
                                    }}
                                >
                                    {page.ballots.map((ballot) => (
                                        <Box
                                            className="ballot-box-log-mobile-row"
                                            component="li"
                                            key={ballot.tracking_id}
                                            sx={{
                                                p: 2,
                                                borderTop: "1px solid #DFE6EF",
                                                borderLeft: ownIds.has(ballot.tracking_id)
                                                    ? "3px solid #34D399"
                                                    : "3px solid transparent",
                                                bgcolor: ownIds.has(ballot.tracking_id)
                                                    ? "#F0FDF8"
                                                    : undefined,
                                            }}
                                        >
                                            <Stack
                                                className="ballot-box-log-mobile-heading"
                                                direction="row"
                                                alignItems="center"
                                                gap={1}
                                            >
                                                <ButtonBase
                                                    className="ballot-box-log-mobile-check-button"
                                                    onClick={() => onOpen(ballot)}
                                                    sx={{
                                                        p: 0,
                                                        fontWeight: 700,
                                                        color: "text.primary",
                                                        letterSpacing: 0.6,
                                                    }}
                                                >
                                                    {code(
                                                        shortBallotId(ballot.tracking_id),
                                                        parsed?.trackingId
                                                    )}
                                                </ButtonBase>
                                                {ownIds.has(ballot.tracking_id) && yours}
                                                <Box
                                                    className="ballot-box-log-mobile-arrow-container"
                                                    sx={{ml: "auto"}}
                                                >
                                                    <FontAwesomeIcon
                                                        className="ballot-box-log-mobile-arrow-icon"
                                                        icon={faChevronRight}
                                                        style={{fontSize: 12, color: "#64748B"}}
                                                    />
                                                </Box>
                                            </Stack>
                                            <Stack
                                                className="ballot-box-log-mobile-details"
                                                direction="row"
                                                alignItems="center"
                                                useFlexGap
                                                flexWrap="wrap"
                                                gap={1}
                                                sx={{mt: 0.75}}
                                            >
                                                {stageTag(ballot)}
                                                <PseudonymChip className="ballot-box-log-mobile-pseudonym">
                                                    {code(
                                                        pseudonymDisplay(ballot.pseudonym),
                                                        parsed?.pseudonym
                                                    )}
                                                </PseudonymChip>
                                                <Typography
                                                    className="ballot-box-log-mobile-time-channel"
                                                    variant="caption"
                                                    color="text.secondary"
                                                    sx={{m: 0}}
                                                >
                                                    {formatTime(ballot.last_step_at)}
                                                    {ballot.voting_channel
                                                        ? ` · ${channel(ballot.voting_channel)}`
                                                        : ""}
                                                </Typography>
                                            </Stack>
                                        </Box>
                                    ))}
                                </Box>
                            </>
                        )}
                        <Stack
                            className="ballot-box-log-pagination"
                            direction="row"
                            useFlexGap
                            flexWrap="wrap"
                            gap={1.5}
                            alignItems="center"
                            sx={{px: 2.5, py: 1.5, borderTop: "1px solid #DFE6EF"}}
                        >
                            {timeZoneLabel && (
                                <Typography
                                    className="ballot-box-log-timezone"
                                    variant="caption"
                                    color="text.secondary"
                                    sx={{m: 0, mr: "auto"}}
                                >
                                    {t("ballotPrivacy.timesIn", {zone: timeZoneLabel})}
                                </Typography>
                            )}
                            <TextField
                                className="ballot-box-log-page-size"
                                select
                                size="small"
                                label={t("ballotPrivacy.rows")}
                                value={limit}
                                onChange={(event) => onLimit(Number(event.target.value))}
                                sx={{minWidth: 130, width: 140, ml: {sm: "auto"}}}
                            >
                                {[10, 25, 50].map((size) => (
                                    <MenuItem
                                        className="ballot-box-log-page-size-option"
                                        key={size}
                                        value={size}
                                    >
                                        {size}
                                    </MenuItem>
                                ))}
                            </TextField>
                            <Tooltip
                                className="ballot-box-log-previous-tooltip"
                                title={t("ballotPrivacy.previous")}
                            >
                                <span className="ballot-box-log-previous-wrapper">
                                    <IconButton
                                        className="ballot-box-log-previous-button"
                                        aria-label={t("ballotPrivacy.previous")}
                                        disabled={!canPrevious}
                                        onClick={onPrevious}
                                    >
                                        <FontAwesomeIcon
                                            className="ballot-box-log-previous-icon"
                                            icon={faChevronLeft}
                                            style={{fontSize: 14}}
                                        />
                                    </IconButton>
                                </span>
                            </Tooltip>
                            <Tooltip
                                className="ballot-box-log-next-tooltip"
                                title={t("ballotPrivacy.next")}
                            >
                                <span className="ballot-box-log-next-wrapper">
                                    <IconButton
                                        className="ballot-box-log-next-button"
                                        aria-label={t("ballotPrivacy.next")}
                                        disabled={!page.next_cursor}
                                        onClick={onNext}
                                    >
                                        <FontAwesomeIcon
                                            className="ballot-box-log-next-icon"
                                            icon={faChevronRight}
                                            style={{fontSize: 14}}
                                        />
                                    </IconButton>
                                </span>
                            </Tooltip>
                        </Stack>
                    </>
                )}
            </Paper>
            <Dialog
                className="ballot-box-log-help-dialog"
                open={help}
                onClose={() => setHelp(false)}
                fullWidth
                maxWidth="sm"
                aria-labelledby="ballot-log-help"
            >
                <DialogTitle className="ballot-box-log-help-title" id="ballot-log-help">
                    {t("ballotPrivacy.help")}
                </DialogTitle>
                <DialogContent className="ballot-box-log-help-content">
                    <Stack className="ballot-box-log-help-sections" spacing={2}>
                        {["pseudonyms", "signatures", "checks"].map((key) => (
                            <Box className="ballot-box-log-help-section" key={key}>
                                <Typography
                                    className="ballot-box-log-help-section-title"
                                    fontWeight={700}
                                    sx={{m: 0}}
                                >
                                    {t(`ballotPrivacy.helpSections.${key}.title`)}
                                </Typography>
                                <Typography
                                    className="ballot-box-log-help-section-text"
                                    variant="body2"
                                    color="text.secondary"
                                    sx={{m: 0}}
                                >
                                    {t(`ballotPrivacy.helpSections.${key}.text`)}
                                </Typography>
                            </Box>
                        ))}
                    </Stack>
                </DialogContent>
                <DialogActions className="ballot-box-log-help-actions">
                    <Button
                        className="ballot-box-log-help-close-button"
                        onClick={() => setHelp(false)}
                    >
                        {t("ballotPrivacy.close")}
                    </Button>
                </DialogActions>
            </Dialog>
            <Dialog
                className="ballot-box-log-ballot-id-help-dialog"
                open={ballotHelp}
                onClose={() => setBallotHelp(false)}
                fullWidth
                maxWidth="sm"
                aria-labelledby="log-ballot-id-help"
            >
                <DialogTitle
                    className="ballot-box-log-ballot-id-help-title"
                    id="log-ballot-id-help"
                >
                    {t("confirmationScreen.ballotIdHelpDialog.title")}
                </DialogTitle>
                <DialogContent className="ballot-box-log-ballot-id-help-content">
                    {stringToHtml(t("confirmationScreen.ballotIdHelpDialog.content"))}
                </DialogContent>
                <DialogActions className="ballot-box-log-ballot-id-help-actions">
                    <Button
                        className="ballot-box-log-ballot-id-help-close-button"
                        onClick={() => setBallotHelp(false)}
                    >
                        {t("ballotPrivacy.close")}
                    </Button>
                </DialogActions>
            </Dialog>
        </Stack>
    )
}
const readSearch = (search: string): string | null => {
    const parsed = parseBallotLogSearch(search)
    return parsed && searchesAnything(parsed) ? parsed.query : null
}
/** What is asked of the server: nothing for the policy's ID, which no ballot has. */
const searchSent = (search: string, policyId: string | undefined): string | null =>
    isPolicyIdTyped(search, policyId) ? null : readSearch(search)
export const BallotBoxLog: React.FC<{
    /** The event's Vitrine policy, which names the explorers of its entries. */
    policy?: Policy
    electionEventId: string
    electionId?: string
    formatTime: (timestamp: string) => string
    timeZoneLabel?: string
}> = ({policy, electionEventId, electionId, formatTime, timeZoneLabel}) => {
    const policyId = policy?.id
    const [search, setSearch] = useState("")
    // What is asked of the server: the typed search once it reads as one, or
    // null while it does not, so nothing unreadable is sent.
    const [prefix, setPrefix] = useState<string | null>("")
    const [limit, setLimit] = useState(10)
    const [cursors, setCursors] = useState<Array<string | null>>([null])
    const [selected, setSelected] = useState<Pick<IBallotLogRow, "tracking_id" | "election_id">>()
    useEffect(() => {
        const timer = setTimeout(() => {
            setPrefix(searchSent(search, policyId))
            setCursors([null])
        }, 300)
        return () => clearTimeout(timer)
    }, [search, policyId])
    useEffect(() => {
        setSearch("")
        setPrefix("")
        setCursors([null])
        setSelected(undefined)
    }, [electionEventId, electionId])
    // A short ID is looked up in a bucket of the record's servers; only when
    // none lists buckets does the election's server see it.
    const parsedPrefix = parseBallotLogSearch(prefix ?? "")
    const look =
        parsedPrefix && parsedPrefix.pseudonym === null && parsedPrefix.trackingId
            ? shortIdToLook(parsedPrefix.trackingId)
            : undefined
    const bucket = useBucketMatches({
        look,
        eventId: electionEventId,
        applicable: !!policy,
        policy,
        settled: true,
    })
    const bucketBusy = bucket.status === EShortIdStatus.LOADING
    const listed =
        bucket.status === EShortIdStatus.DONE &&
        (bucket.result?.kind === EShortIdKind.LISTED || bucket.result?.kind === EShortIdKind.MANY)
            ? bucket.result
            : undefined
    const list = useQuery<{ballot_box_log: {payload: IBallotLogPage}}>(BALLOT_BOX_LOG, {
        variables: {
            electionEventId,
            electionId,
            prefix,
            cursor: cursors[cursors.length - 1],
            limit,
        },
        skip: prefix === null || bucketBusy || !!listed,
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
                loading={list.loading || bucketBusy || searchSent(search, policyId) !== prefix}
                error={!!list.error}
                search={search}
                onSearch={setSearch}
                policyIdTyped={isPolicyIdTyped(search, policyId)}
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
                timeZoneLabel={timeZoneLabel}
                {...(listed
                    ? {
                          shortIds: {
                              result: listed,
                              onOpen: (match: IShortIdMatch) =>
                                  setSelected({
                                      tracking_id: match.trackingId,
                                      election_id: match.election || electionId || "",
                                  }),
                              onNarrow: (typed: string) => setSearch(groupBallotId(typed)),
                          },
                      }
                    : {})}
                shortIdServerSaw={bucket.status === EShortIdStatus.DONE && !listed && !!look}
            />
            {selected && (
                <BallotBoxEntry
                    electionEventId={electionEventId}
                    electionId={selected.election_id}
                    ballotId={selected.tracking_id}
                    policy={policy}
                    onClose={() => setSelected(undefined)}
                />
            )}
        </>
    )
}
