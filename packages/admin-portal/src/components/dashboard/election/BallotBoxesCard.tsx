// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useMemo, useState} from "react"
import {useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Chip,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    Typography,
} from "@mui/material"
import {EBallotBoxSealPolicy, EVotingStatus, VotingStatusChannel} from "@sequentech/ui-core"
import {BallotHashCopyButton, theme} from "@sequentech/ui-essentials"
import CardChart from "../charts/Charts"
import {outlinedWarningChipSx, wrappingChipSx} from "./ballotBoxChip"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useEventPresentation} from "@/hooks/useZonedFormat"
import {intlLanguage, useZonedTime} from "@/hooks/useZonedTime"
import {formatList, shortHash} from "@/components/signing/format"
import {
    GET_BALLOT_BOX_AREA_NAMES,
    GET_BALLOT_BOX_AREAS,
    GET_BALLOT_BOX_SEALS,
} from "@/queries/GetBallotBoxSeals"
import {sealText} from "@/services/ballotBoxSealErrors"
import {SealRecordLink} from "./SealRecordLink"
import {electionSealChannels, sealProgress} from "@/services/sealOnStop"

import {
    EBallotBoxClosedByKind,
    EBallotBoxSealStatus,
    EBallotBoxWaitingReason,
    parseWaitingReason,
    type GetBallotBoxAreaNamesQuery,
    type GetBallotBoxAreaNamesQueryVariables,
    type GetBallotBoxAreasQuery,
    type GetBallotBoxAreasQueryVariables,
    type GetBallotBoxSealsQuery,
    type GetBallotBoxSealsQueryVariables,
    type IBallotBoxClosedBy,
    type IBallotBoxSeal,
} from "@/types/ballotBoxSeal"

/** The categories of a failed attempt the sealer records (`error:<category>`). */
export const WAITING_ERROR_CATEGORIES = [
    "board",
    "census",
    "keystore",
    "storage",
    "settings",
    "ballots",
    "database",
    "other",
]

/** What the card shows for one ballot box. */
export enum EBallotBoxRowStatus {
    /** No seal row: voting has not closed. */
    OPEN = "open",
    /** Closed; sealed when the grace period ends. */
    SEALING = "sealing",
    /** Past its deadline and the sealer is on it (it takes up to a minute). */
    DUE = "due",
    /** Past its deadline and held: a channel, Datafix votes, an error, or no sealer running. */
    OVERDUE = "overdue",
    /** Sealed and locked; its entry on the bulletin board is being posted. */
    PUBLISHING = "publishing",
    /** Sealed and on the bulletin board. */
    SEALED = "sealed",
    /** Not sealed: an incident. The ballot box stays locked. */
    FAILED = "failed",
}

/**
 * How long after its deadline, or after the sealer's last attempt, a pending
 * box counts as overdue: the beat tries every minute.
 */
export const STALE_ATTEMPT_MS = 3 * 60_000

/** Whether a pending box's last attempt (or its deadline, if never tried) is stale. */
const isStale = (seal: IBallotBoxSeal, now: Date): boolean =>
    now.getTime() - new Date(seal.last_attempt_at ?? seal.grace_deadline).getTime() >
    STALE_ATTEMPT_MS

export const ballotBoxRowStatus = (
    seal: IBallotBoxSeal | undefined,
    now: Date
): EBallotBoxRowStatus => {
    if (!seal) return EBallotBoxRowStatus.OPEN
    switch (seal.status) {
        case EBallotBoxSealStatus.PENDING: {
            if (now.getTime() < new Date(seal.grace_deadline).getTime()) {
                return EBallotBoxRowStatus.SEALING
            }
            const waiting = parseWaitingReason(seal.waiting_reason)?.reason
            const held =
                waiting === EBallotBoxWaitingReason.CHANNEL_OPEN ||
                waiting === EBallotBoxWaitingReason.CHANNEL_NOT_ENABLED ||
                waiting === EBallotBoxWaitingReason.CHANNEL_HAS_BALLOTS ||
                waiting === EBallotBoxWaitingReason.DATAFIX_VOTES ||
                waiting === EBallotBoxWaitingReason.ERROR
            return held || isStale(seal, now)
                ? EBallotBoxRowStatus.OVERDUE
                : EBallotBoxRowStatus.DUE
        }
        case EBallotBoxSealStatus.SEALED:
            return EBallotBoxRowStatus.PUBLISHING
        case EBallotBoxSealStatus.PUBLISHED:
            return EBallotBoxRowStatus.SEALED
        default:
            return EBallotBoxRowStatus.FAILED
    }
}

const STATUS_CHIP: Record<
    EBallotBoxRowStatus,
    {color: "default" | "success" | "warning" | "error"; variant: "filled" | "outlined"}
> = {
    [EBallotBoxRowStatus.OPEN]: {color: "default", variant: "outlined"},
    [EBallotBoxRowStatus.SEALING]: {color: "default", variant: "outlined"},
    [EBallotBoxRowStatus.DUE]: {color: "default", variant: "outlined"},
    [EBallotBoxRowStatus.OVERDUE]: {color: "warning", variant: "outlined"},
    [EBallotBoxRowStatus.PUBLISHING]: {color: "warning", variant: "outlined"},
    [EBallotBoxRowStatus.SEALED]: {color: "success", variant: "filled"},
    [EBallotBoxRowStatus.FAILED]: {color: "error", variant: "filled"},
}

interface IBallotBoxRow {
    areaId: string
    areaName: string
    seal?: IBallotBoxSeal
}

/** The latest of some ISO times. */
const latest = (values: Array<string | null | undefined>): string | undefined =>
    values
        .filter((value): value is string => !!value)
        .reduce<
            string | undefined
        >((max, value) => (!max || new Date(value) > new Date(max) ? value : max), undefined)

/** The election as the card reads its voting status before the close. */
export interface IBallotBoxesElection {
    status?: unknown
    voting_channels?: unknown
}

export interface BallotBoxesCardProps {
    electionEventId?: string | null
    electionId?: string | null
    /** The election's status and channels: what the card says before the close. */
    election?: IBallotBoxesElection | null
    /**
     * The time the statuses are computed at. By default the current time,
     * which ticks while a ballot box waits for its seal, so a pending row
     * moves to "waiting" once its grace period ends.
     */
    now?: Date
}

/**
 * Ballot boxes (VOTE-FREEZE): one row per ballot box of the election with its seal,
 * shown on the election's Dashboard when the event seals its ballot boxes at
 * close. Nothing is shown when it doesn't.
 */
export const BallotBoxesCard: React.FC<BallotBoxesCardProps> = ({
    electionEventId,
    electionId,
    election,
    now,
}) => {
    const presentation = useEventPresentation(electionEventId)
    const sealAtClose = presentation?.ballot_box_seal_policy === EBallotBoxSealPolicy.SEAL_AT_CLOSE
    if (!sealAtClose || !electionEventId || !electionId) return null
    return (
        <BallotBoxesTable
            electionEventId={electionEventId}
            electionId={electionId}
            election={election}
            now={now}
        />
    )
}

/** How often the card's clock ticks while a ballot box is pending. */
const PENDING_TICK_MS = 30_000

const BallotBoxesTable: React.FC<{
    electionEventId: string
    electionId: string
    election?: IBallotBoxesElection | null
    now?: Date
}> = ({electionEventId, electionId, election, now: fixedNow}) => {
    const {t, i18n} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const pollInterval = globalSettings.QUERY_POLL_INTERVAL_MS
    const [clock, setClock] = useState(() => new Date())
    const now = fixedNow ?? clock
    const zonedTime = useZonedTime(now)

    const {data: sealsData, error: sealsError} = useQuery<
        GetBallotBoxSealsQuery,
        GetBallotBoxSealsQueryVariables
    >(GET_BALLOT_BOX_SEALS, {variables: {electionEventId, electionIds: [electionId]}, pollInterval})

    const seals = useMemo(
        () => sealsData?.sequent_backend_ballot_box_seal ?? [],
        [sealsData?.sequent_backend_ballot_box_seal]
    )

    // Polling doesn't re-render an unchanged pending row, so the clock ticks
    // while one waits for the end of its grace period.
    const anyPending = seals.some((seal) => seal.status === EBallotBoxSealStatus.PENDING)
    useEffect(() => {
        if (fixedNow || !anyPending) return
        setClock(new Date())
        const timer = window.setInterval(() => setClock(new Date()), PENDING_TICK_MS)
        return () => window.clearInterval(timer)
    }, [fixedNow, anyPending])

    // The election's ballot boxes before the close: the areas of its published
    // ballot styles, the set the close seals. Not reading them only drops the
    // Open rows; the seal rows are listed anyway.
    const {data: stylesData} = useQuery<GetBallotBoxAreasQuery, GetBallotBoxAreasQueryVariables>(
        GET_BALLOT_BOX_AREAS,
        {variables: {electionEventId, electionId}}
    )
    const areaIds = useMemo(
        () =>
            Array.from(
                new Set(
                    (stylesData?.sequent_backend_ballot_style ?? [])
                        .map((style) => style.area_id)
                        .filter((id): id is string => !!id)
                )
            ).sort(),
        [stylesData]
    )
    const {data: namesData} = useQuery<
        GetBallotBoxAreaNamesQuery,
        GetBallotBoxAreaNamesQueryVariables
    >(GET_BALLOT_BOX_AREA_NAMES, {variables: {areaIds}, skip: !areaIds.length})

    // One row per ballot box: every area of the election, merged with the seal
    // rows the close inserts (one per ballot box it seals).
    const rows = useMemo<IBallotBoxRow[]>(() => {
        const names = new Map<string, string>(
            (namesData?.sequent_backend_area ?? []).map((area) => [
                String(area.id),
                area.name ?? String(area.id),
            ])
        )
        const ids = new Set([...areaIds, ...seals.map((seal) => seal.area_id)])
        return Array.from(ids, (areaId) => {
            const seal = seals.find((row) => row.area_id === areaId)
            return {
                areaId,
                areaName: seal?.area_name ?? seal?.area?.name ?? names.get(areaId) ?? areaId,
                seal,
            }
        }).sort((a, b) => a.areaName.localeCompare(b.areaName, i18n.language))
    }, [areaIds, namesData, seals, i18n.language])

    const closed = latest(seals.map((seal) => seal.closed_at))
    const deadline = latest(seals.map((seal) => seal.grace_deadline))
    const closeRow = seals.find((seal) => seal.closed_at === closed)

    const channelNames = (channels: VotingStatusChannel[]) =>
        new Intl.ListFormat(intlLanguage(i18n.language), {
            style: "long",
            type: "conjunction",
        }).format(channels.map((channel) => t(`publish.dialog.channel.${channel}`)))

    /** What the card says before any ballot box has a seal row, from the election's status. */
    const beforeClose = (): string => {
        if (!election) return t("dashboard.ballotBoxes.beforeClose")
        const all = electionSealChannels(election)
        const channels = all.filter(({enabled}) => enabled)
        const progress = sealProgress(all)
        // A channel the Post doesn't enable that is open or ran holds the
        // seal: say so, whatever the enabled channels' state.
        const notEnabled = progress.notEnabled
            .map((channel) =>
                t("publish.dialog.sealNotEnabled", {
                    channel: t(`publish.dialog.channel.${channel}`),
                })
            )
            .join(" ")
        const closed = channels.some(({status}) => status === EVotingStatus.CLOSED)
        if (closed && progress.holding.length) {
            return [
                t("dashboard.ballotBoxes.holding", {
                    count: progress.holding.length,
                    channels: channelNames(progress.holding),
                }),
                notEnabled,
            ]
                .filter(Boolean)
                .join(" ")
        }
        if (notEnabled) return notEnabled
        const open = channels.filter(({status}) => status === EVotingStatus.OPEN)
        if (open.length) {
            return t("dashboard.ballotBoxes.openOn", {
                channels: channelNames(open.map(({channel}) => channel)),
            })
        }
        if (channels.some(({status}) => status === EVotingStatus.PAUSED)) {
            return t("dashboard.ballotBoxes.paused")
        }
        if (channels.every(({status}) => status === EVotingStatus.NOT_STARTED)) {
            return t("dashboard.ballotBoxes.notStarted")
        }
        return t("dashboard.ballotBoxes.beforeClose")
    }

    const pending = seals.some((seal) => seal.status === EBallotBoxSealStatus.PENDING)
    const summary = sealsError
        ? t("dashboard.ballotBoxes.loadError")
        : !seals.length
          ? beforeClose()
          : seals.some((seal) => seal.status === EBallotBoxSealStatus.FAILED)
            ? t("dashboard.ballotBoxes.failed", {closed: zonedTime(closed)})
            : !pending
              ? t("dashboard.ballotBoxes.sealed", {closed: zonedTime(closed)})
              : deadline === closed
                ? t("dashboard.ballotBoxes.sealingNow", {closed: zonedTime(closed)})
                : deadline && now.getTime() < new Date(deadline).getTime()
                  ? t("dashboard.ballotBoxes.sealing", {
                        closed: zonedTime(closed),
                        deadline: zonedTime(deadline),
                    })
                  : t("dashboard.ballotBoxes.sealingPastGrace", {
                        closed: zonedTime(closed),
                        deadline: zonedTime(deadline),
                    })

    const closedByText = (closedBy?: IBallotBoxClosedBy | null): string | null => {
        switch (closedBy?.kind) {
            case EBallotBoxClosedByKind.SIGNED: {
                const names = (closedBy.signers ?? []).map((signer) => signer.name)
                if (!names.length) return null
                return t("dashboard.ballotBoxes.closedBySignatures", {
                    names: formatList(names, intlLanguage(i18n.language)),
                    code: closedBy.signing_code ?? "",
                })
            }
            case EBallotBoxClosedByKind.USER:
                return closedBy.username
                    ? t("dashboard.ballotBoxes.closedByUser", {username: closedBy.username})
                    : null
            case EBallotBoxClosedByKind.SCHEDULED:
                return t("dashboard.ballotBoxes.closedBySchedule")
            default:
                return null
        }
    }
    const closedBy = closedByText(closeRow?.closed_by)

    const statusLabel = (status: EBallotBoxRowStatus, seal?: IBallotBoxSeal): string =>
        status === EBallotBoxRowStatus.SEALING
            ? t("dashboard.ballotBoxes.status.sealing", {time: zonedTime(seal?.grace_deadline)})
            : t(`dashboard.ballotBoxes.status.${status}`)

    /** Why a ballot box is in its state, shown under its status (not only on hover). */
    const statusHelp = (status: EBallotBoxRowStatus, seal?: IBallotBoxSeal): string => {
        if (!seal) return ""
        switch (status) {
            case EBallotBoxRowStatus.PUBLISHING:
                return t("dashboard.ballotBoxes.help.publishing")
            case EBallotBoxRowStatus.FAILED:
                return sealText(t, seal.failure_reason)
            case EBallotBoxRowStatus.DUE:
                return t("dashboard.ballotBoxes.why.due")
            case EBallotBoxRowStatus.OVERDUE: {
                const waiting = parseWaitingReason(seal.waiting_reason)
                if (waiting?.reason === EBallotBoxWaitingReason.CHANNEL_OPEN && waiting.detail) {
                    return t("dashboard.ballotBoxes.why.channelOpen", {
                        channel: t(`publish.dialog.channel.${waiting.detail}`),
                    })
                }
                if (
                    waiting?.reason === EBallotBoxWaitingReason.CHANNEL_NOT_ENABLED &&
                    waiting.detail
                ) {
                    return t("dashboard.ballotBoxes.why.channelNotEnabled", {
                        channel: t(`publish.dialog.channel.${waiting.detail}`),
                    })
                }
                if (
                    waiting?.reason === EBallotBoxWaitingReason.CHANNEL_HAS_BALLOTS &&
                    waiting.detail
                ) {
                    return t("dashboard.ballotBoxes.why.channelHasBallots", {
                        channel: t(`publish.dialog.channel.${waiting.detail}`),
                    })
                }
                if (waiting?.reason === EBallotBoxWaitingReason.DATAFIX_VOTES) {
                    return t("dashboard.ballotBoxes.why.datafixVotes", {
                        count: Number(waiting.detail) || 0,
                    })
                }
                if (waiting?.reason === EBallotBoxWaitingReason.ERROR) {
                    // A category code, never the error's own text (it may hold
                    // hosts or URLs); the service log has the details.
                    return WAITING_ERROR_CATEGORIES.includes(waiting.detail ?? "")
                        ? t(`dashboard.ballotBoxes.why.errorCategory.${waiting.detail}`)
                        : t("dashboard.ballotBoxes.why.errorCategory.other")
                }
                return seal.last_attempt_at
                    ? t("dashboard.ballotBoxes.why.stale", {time: zonedTime(seal.last_attempt_at)})
                    : t("dashboard.ballotBoxes.why.notTried")
            }
            default:
                return ""
        }
    }

    const copyLabels = {
        copy: t("dashboard.ballotBoxes.copyHash"),
        copied: t("dashboard.ballotBoxes.copied"),
        error: t("dashboard.ballotBoxes.copyError"),
    }
    const number = (value?: number | null) =>
        value === null || value === undefined ? "—" : value.toLocaleString(i18n.language)

    return (
        <Box sx={{maxWidth: 1024, marginX: "auto", marginTop: 2}}>
            <CardChart title={t("dashboard.ballotBoxes.title")}>
                {sealsError ? (
                    // Unknown, not "Open": the seals could not be read.
                    <Alert severity="warning" sx={{mt: 1}}>
                        {summary}
                    </Alert>
                ) : (
                    <>
                        <Typography variant="body2" sx={{mt: 1}}>
                            {summary}
                        </Typography>
                        {closedBy ? (
                            <Typography variant="body2" color={theme.palette.customGrey.main}>
                                {closedBy}
                            </Typography>
                        ) : null}
                        {rows.length ? (
                            <Box sx={{overflowX: "auto", position: "relative", mt: 1}}>
                                <Table
                                    size="small"
                                    sx={{"& th": {whiteSpace: "nowrap"}}}
                                    aria-label={t("dashboard.ballotBoxes.title")}
                                >
                                    <TableHead>
                                        <TableRow>
                                            <TableCell>
                                                {t("dashboard.ballotBoxes.column.area")}
                                            </TableCell>
                                            <TableCell>
                                                {t("dashboard.ballotBoxes.column.status")}
                                            </TableCell>
                                            <TableCell align="right">
                                                {t("dashboard.ballotBoxes.column.inTheBox")}
                                            </TableCell>
                                            <TableCell align="right">
                                                {t("dashboard.ballotBoxes.column.counted")}
                                            </TableCell>
                                            <TableCell>
                                                {t("dashboard.ballotBoxes.column.sealedAt")}
                                            </TableCell>
                                            <TableCell>
                                                {t("dashboard.ballotBoxes.column.sealHash")}
                                            </TableCell>
                                            <TableCell>
                                                {t("dashboard.ballotBoxes.column.record")}
                                            </TableCell>
                                        </TableRow>
                                    </TableHead>
                                    <TableBody>
                                        {rows.map(({areaId, areaName, seal}) => {
                                            const status = ballotBoxRowStatus(seal, now)
                                            const chip = STATUS_CHIP[status]
                                            const help = statusHelp(status, seal)
                                            const isSealed =
                                                status === EBallotBoxRowStatus.SEALED ||
                                                status === EBallotBoxRowStatus.PUBLISHING
                                            return (
                                                <TableRow key={areaId}>
                                                    <TableCell>{areaName}</TableCell>
                                                    <TableCell sx={{minWidth: 190}}>
                                                        <Chip
                                                            size="small"
                                                            label={statusLabel(status, seal)}
                                                            color={chip.color}
                                                            variant={chip.variant}
                                                            sx={[
                                                                wrappingChipSx,
                                                                chip.color === "warning" &&
                                                                    chip.variant === "outlined" &&
                                                                    outlinedWarningChipSx,
                                                            ]}
                                                        />
                                                        {help ? (
                                                            <Typography
                                                                variant="caption"
                                                                display="block"
                                                                sx={{maxWidth: 260, mt: 0.5}}
                                                                color={
                                                                    status ===
                                                                    EBallotBoxRowStatus.FAILED
                                                                        ? "error"
                                                                        : "text.secondary"
                                                                }
                                                            >
                                                                {help}
                                                            </Typography>
                                                        ) : null}
                                                    </TableCell>
                                                    <TableCell align="right">
                                                        {isSealed
                                                            ? number(seal?.ballots_in_box)
                                                            : "—"}
                                                    </TableCell>
                                                    <TableCell align="right">
                                                        {isSealed
                                                            ? number(seal?.ballots_counted)
                                                            : "—"}
                                                    </TableCell>
                                                    <TableCell sx={{whiteSpace: "nowrap"}}>
                                                        {isSealed && seal?.sealed_at
                                                            ? zonedTime(seal.sealed_at)
                                                            : "—"}
                                                    </TableCell>
                                                    <TableCell sx={{whiteSpace: "nowrap"}}>
                                                        {isSealed && seal?.seal_hash ? (
                                                            <Box
                                                                component="span"
                                                                sx={{
                                                                    display: "inline-flex",
                                                                    alignItems: "center",
                                                                }}
                                                            >
                                                                <Box
                                                                    component="span"
                                                                    sx={{
                                                                        fontFamily:
                                                                            "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
                                                                    }}
                                                                    title={seal.seal_hash}
                                                                >
                                                                    {shortHash(seal.seal_hash)}
                                                                </Box>
                                                                <BallotHashCopyButton
                                                                    hash={seal.seal_hash}
                                                                    copyLabels={copyLabels}
                                                                />
                                                            </Box>
                                                        ) : (
                                                            "—"
                                                        )}
                                                    </TableCell>
                                                    <TableCell sx={{whiteSpace: "nowrap"}}>
                                                        {status === EBallotBoxRowStatus.SEALED &&
                                                        seal ? (
                                                            <SealRecordLink
                                                                electionEventId={electionEventId}
                                                                seal={seal}
                                                                areaName={areaName}
                                                            />
                                                        ) : status ===
                                                          EBallotBoxRowStatus.PUBLISHING ? (
                                                            t("dashboard.ballotBoxes.notYet")
                                                        ) : (
                                                            "—"
                                                        )}
                                                    </TableCell>
                                                </TableRow>
                                            )
                                        })}
                                    </TableBody>
                                </Table>
                            </Box>
                        ) : null}
                        {rows.some(
                            ({seal}) =>
                                seal?.ballots_counted !== undefined &&
                                seal?.ballots_counted !== null
                        ) ? (
                            <Typography
                                variant="caption"
                                display="block"
                                color="text.secondary"
                                sx={{mt: 1}}
                            >
                                {t("dashboard.ballotBoxes.help.counted")}
                            </Typography>
                        ) : null}
                    </>
                )}
            </CardChart>
        </Box>
    )
}

export default BallotBoxesCard
