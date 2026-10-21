// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useId, useMemo} from "react"
import {Alert, Box, Button, Chip, Stack, Typography} from "@mui/material"
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome"
import {
    faCircleCheck,
    faCircleXmark,
    faClock,
    faDownload,
    faLock,
} from "@fortawesome/free-solid-svg-icons"
import {useTranslation} from "react-i18next"
import {verifyBallotLogProof, type BallotPrivacyCheckStatus} from "@sequentech/ui-core"
import type {BallotLogEvidence, BallotLogTrust} from "@sequentech/ui-essentials"
import {useZonedFormat} from "@/hooks/useZonedFormat"
import {shortened} from "./logEntryDetails"

interface ILogEntryMetadata {
    id: string
    kind: "BallotReceived" | "CastVote"
    timestamp: number
    description: string
    voter_public_key: string | null
}

export interface IAdminBallotEvidence extends BallotLogEvidence {
    /** Display metadata only; checks always consume the canonical signed bytes. */
    log_entries?: ILogEntryMetadata[]
}

export interface IBallotLogSelection {
    id: number | string
    statement_kind?: string | null
    statement_timestamp?: number | null
}

const GROUPS = [
    {kind: "BallotReceived", checks: ["binding", "received", "ballotId"], label: "received"},
    {kind: "CastVote", checks: ["cast"], label: "cast"},
] as const
const STATUS_ICONS = {passed: faCircleCheck, failed: faCircleXmark, pending: faClock}
const STATUS_COLORS = {passed: "success.main", failed: "error.main", pending: "text.secondary"}

const combinedStatus = (statuses: BallotPrivacyCheckStatus[]): BallotPrivacyCheckStatus =>
    statuses.includes("failed") ? "failed" : statuses.includes("pending") ? "pending" : "passed"

export const BallotEntryEvidence: React.FC<{
    proof: IAdminBallotEvidence
    trust: BallotLogTrust
    selectedEntry: IBallotLogSelection
    zone: string
    onRetry: () => void
}> = ({proof, trust, selectedEntry, zone, onRetry}) => {
    const {t, i18n} = useTranslation()
    const format = useZonedFormat(zone, {seconds: true})
    const headingId = useId()
    const checked = useMemo(() => {
        try {
            return verifyBallotLogProof(trust, proof)
        } catch {
            return null
        }
    }, [trust, proof])
    const state = combinedStatus(
        GROUPS.flatMap(({checks}) => checks.map((key) => checked?.checks[key] ?? "failed"))
    )
    const voterKey = proof.log_entries?.find(
        ({kind}) => kind === "BallotReceived"
    )?.voter_public_key
    const download = () => {
        const blob = new Blob([JSON.stringify({trust, proof}, null, 2)], {type: "application/json"})
        const url = URL.createObjectURL(blob)
        const link = document.createElement("a")
        link.href = url
        link.download = `ballot-${proof.ballot_id.replace(/[^A-Z0-9-]/gi, "")}-signed-entries.json`
        link.click()
        setTimeout(() => URL.revokeObjectURL(url), 0)
    }
    return (
        <Stack spacing={1.5} sx={{minWidth: 0}}>
            <Box
                component="dl"
                sx={{
                    display: "grid",
                    gridTemplateColumns: {xs: "1fr", sm: "repeat(2, minmax(0, 1fr))"},
                    gap: 1.5,
                    m: 0,
                }}
            >
                {[
                    {label: t("ballotPrivacy.admin.voterKey"), value: voterKey},
                    {label: t("ballotPrivacy.boxKey"), value: trust.ballot_box_key.public_key},
                ].map(({label, value}) => (
                    <Box
                        key={label}
                        sx={{
                            display: "grid",
                            gridTemplateColumns: {xs: "1fr", sm: "8rem minmax(0, 1fr)"},
                            columnGap: 2,
                            minWidth: 0,
                        }}
                    >
                        <Typography component="dt" variant="body2" color="text.secondary">
                            {label}
                        </Typography>
                        <Typography
                            component="dd"
                            variant="body2"
                            title={value ?? undefined}
                            sx={{
                                m: 0,
                                fontFamily: '"DejaVu Sans Mono", ui-monospace, monospace',
                                fontSize: 12.5,
                                overflowWrap: "anywhere",
                            }}
                        >
                            {value ? shortened(value, 16, 4) : "—"}
                        </Typography>
                    </Box>
                ))}
            </Box>
            {state !== "passed" ? (
                <Alert severity={state === "failed" ? "error" : "info"}>
                    <Typography variant="body2" fontWeight={600} sx={{m: 0}}>
                        {t(`ballotPrivacy.${state}`)}
                    </Typography>
                    {t(`ballotPrivacy.${state === "failed" ? "failedHelp" : "pendingHelp"}`)}
                </Alert>
            ) : null}
            <Box
                sx={{
                    display: "flex",
                    flexWrap: "wrap",
                    justifyContent: "space-between",
                    gap: 1,
                    pt: 0.5,
                }}
            >
                <Typography
                    component="h4"
                    id={headingId}
                    variant="body2"
                    color="text.secondary"
                    sx={{m: 0}}
                >
                    {t("ballotPrivacy.admin.entries")}
                </Typography>
                {state === "passed" ? (
                    <Typography variant="caption" color="success.main">
                        {t("ballotPrivacy.passed")}
                    </Typography>
                ) : null}
            </Box>
            <Box
                component="ol"
                aria-labelledby={headingId}
                sx={{
                    listStyle: "none",
                    m: 0,
                    p: 0,
                    border: 1,
                    borderColor: "divider",
                    borderRadius: 1,
                    overflow: "hidden",
                }}
            >
                {GROUPS.map(({kind, checks, label}, index) => {
                    const status = combinedStatus(
                        checks.map((key) => checked?.checks[key] ?? "failed")
                    )
                    const metadata = proof.log_entries?.find((entry) => entry.kind === kind)
                    const current = metadata
                        ? String(selectedEntry.id) === metadata.id
                        : selectedEntry.statement_kind === kind
                    const timestamp =
                        metadata?.timestamp ?? (current ? selectedEntry.statement_timestamp : null)
                    const id = metadata?.id ?? (current ? String(selectedEntry.id) : null)
                    const number =
                        id && Number.isSafeInteger(Number(id))
                            ? Number(id).toLocaleString(i18n.language)
                            : id
                    return (
                        <Box
                            component="li"
                            key={kind}
                            aria-current={current ? "true" : undefined}
                            data-entry-kind={kind}
                            data-check-state={status}
                            data-check={checks.join(" ")}
                            sx={{
                                display: "grid",
                                gridTemplateColumns: "22px minmax(0, 1fr)",
                                columnGap: 1.5,
                                px: 1.5,
                                py: 1.25,
                                borderTop: index ? 1 : 0,
                                borderColor: "divider",
                                bgcolor: current ? "#F8FAFC" : "transparent",
                                boxShadow: current ? "inset 3px 0 0 #0F054C" : "none",
                            }}
                        >
                            <Box sx={{color: STATUS_COLORS[status], pt: 0.25}}>
                                <FontAwesomeIcon
                                    icon={STATUS_ICONS[status]}
                                    aria-label={t(`ballotPrivacy.states.${status}`)}
                                />
                            </Box>
                            <Box sx={{minWidth: 0}}>
                                <Box
                                    sx={{
                                        display: "flex",
                                        flexWrap: "wrap",
                                        alignItems: "baseline",
                                        columnGap: 1.5,
                                        rowGap: 0.5,
                                    }}
                                >
                                    <Typography
                                        component="span"
                                        variant="body2"
                                        fontWeight={600}
                                        sx={{m: 0, lineHeight: 1.5, overflowWrap: "anywhere"}}
                                    >
                                        {t(`ballotPrivacy.admin.${label}Title`, {
                                            ballotId: proof.ballot_id,
                                        })}
                                        {kind === "CastVote" && proof.voting_channel
                                            ? `, ${t(`ballotPrivacy.channels.${proof.voting_channel.toLowerCase()}`, {defaultValue: proof.voting_channel})}`
                                            : null}
                                    </Typography>
                                    {id || timestamp ? (
                                        <Typography variant="caption" color="text.secondary">
                                            {id
                                                ? t("ballotPrivacy.admin.entryNumber", {id: number})
                                                : null}
                                            {id && timestamp ? " · " : null}
                                            {timestamp
                                                ? format.format(timestamp * 1000, zone)
                                                : null}
                                        </Typography>
                                    ) : null}
                                    {current ? (
                                        <Chip
                                            label={t("ballotPrivacy.admin.thisEntry")}
                                            size="small"
                                            sx={{
                                                height: 20,
                                                borderRadius: "100px",
                                                bgcolor: "#0F054C",
                                                color: "#FFFFFF",
                                                fontSize: 11.5,
                                            }}
                                        />
                                    ) : null}
                                </Box>
                                <Typography
                                    component="p"
                                    variant="body2"
                                    sx={{m: 0, mt: 0.25, fontSize: 13.5, lineHeight: 1.5}}
                                >
                                    {t(`ballotPrivacy.admin.${label}Help`)}
                                </Typography>
                                {status !== "passed" ? (
                                    <Typography variant="body2" sx={{color: STATUS_COLORS[status]}}>
                                        {t(`ballotPrivacy.states.${status}`)}
                                    </Typography>
                                ) : null}
                            </Box>
                        </Box>
                    )
                })}
            </Box>
            <Alert
                severity="info"
                icon={<FontAwesomeIcon icon={faLock} />}
                variant="outlined"
                sx={{
                    "bgcolor": "#F7F9FC",
                    "borderColor": "#DFE6EF",
                    "color": "text.primary",
                    "& .MuiAlert-icon": {color: "#65758B", fontSize: 13},
                }}
            >
                {t("ballotPrivacy.admin.neverPublished")}
            </Alert>
            <Stack direction="row" spacing={1} useFlexGap sx={{flexWrap: "wrap"}}>
                <Button
                    variant="text"
                    startIcon={<FontAwesomeIcon icon={faDownload} />}
                    size="small"
                    onClick={download}
                    sx={{p: 0, minHeight: 28}}
                >
                    {t("ballotPrivacy.download")}
                </Button>
                {state === "pending" ? (
                    <Button size="small" onClick={onRetry}>
                        {t("ballotPrivacy.retry")}
                    </Button>
                ) : null}
            </Stack>
        </Stack>
    )
}
