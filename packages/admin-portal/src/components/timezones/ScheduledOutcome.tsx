// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useId, useState} from "react"
import {
    Box,
    Button,
    Chip,
    Popover,
    Stack,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    Typography,
} from "@mui/material"
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline"
import BlockIcon from "@mui/icons-material/Block"
import {useTranslation} from "react-i18next"
import type {TFunction} from "i18next"
import {
    EScheduledOutcomeCheckId,
    EScheduledOutcomeKind,
    type IScheduledOutcomeCheck,
    type IScheduledOutcomeExplanation,
    type IScheduledOutcomeValue,
} from "@sequentech/ui-core"
import {useTimeZoneService, type IAdminTimeZones} from "./timeZoneService"

/** The translation key part of each outcome and check (the wire values are kebab-case). */
export const OUTCOME_KEY: Record<EScheduledOutcomeKind, string> = {
    [EScheduledOutcomeKind.RUNS]: "runs",
    [EScheduledOutcomeKind.RUNS_UNSIGNED]: "runsUnsigned",
    [EScheduledOutcomeKind.REFUSED]: "refused",
    [EScheduledOutcomeKind.WAITING_FOR_INITIALIZATION]: "waitingForInitialization",
}

export const CHECK_KEY: Record<EScheduledOutcomeCheckId, string> = {
    [EScheduledOutcomeCheckId.NEEDS_SIGNATURES]: "needsSignatures",
    [EScheduledOutcomeCheckId.COVERED]: "covered",
    [EScheduledOutcomeCheckId.UNSIGNED_CLOSE]: "unsignedClose",
    [EScheduledOutcomeCheckId.STRICTER_COPY]: "stricterCopy",
    [EScheduledOutcomeCheckId.DEFAULTS]: "defaults",
    [EScheduledOutcomeCheckId.INITIALIZATION]: "initialization",
    [EScheduledOutcomeCheckId.VOTING_CLOSE]: "votingClose",
}

const OUTCOME_COLOR: Record<EScheduledOutcomeKind, "success" | "warning" | "error"> = {
    [EScheduledOutcomeKind.RUNS]: "success",
    [EScheduledOutcomeKind.RUNS_UNSIGNED]: "warning",
    [EScheduledOutcomeKind.REFUSED]: "error",
    [EScheduledOutcomeKind.WAITING_FOR_INITIALIZATION]: "warning",
}

/** White on the warning colour is too faint to read (axe color-contrast): dark text. */
export const WARNING_TEXT: Record<string, {color: string} | undefined> = {
    warning: {color: "common.black"},
}

/** Parameters named `…_at` or `…_date` are RFC 3339 instants: shown in the row's zone. */
const INSTANT_PARAM = /(_at|_date)$/

/** A check value or next step in words: its message key with its parameters. */
export const valueText = (
    value: IScheduledOutcomeValue | null | undefined,
    zone: string,
    service: Pick<IAdminTimeZones, "formatDateTimeZone" | "text">
): string => {
    if (!value) return "–"
    const params: Record<string, unknown> = {}
    for (const [name, raw] of Object.entries(value.params ?? {})) {
        params[name] =
            INSTANT_PARAM.test(name) && typeof raw === "string" && !Number.isNaN(Date.parse(raw))
                ? service.formatDateTimeZone(raw, zone, service.text)
                : Array.isArray(raw)
                  ? raw.join(", ")
                  : raw
    }
    // A number of signatures is plural: i18next reads `count`.
    if (typeof params.signatures === "number" && params.count === undefined) {
        params.count = params.signatures
    }
    return String(service.text.t(value.message_key, params))
}

/**
 * The one-line note of an outcome (design §5a): who authorized it, or what
 * keeps it from running.
 */
export const outcomeNote = (explanation: IScheduledOutcomeExplanation, t: TFunction): string => {
    switch (explanation.outcome) {
        case EScheduledOutcomeKind.RUNS:
            return explanation.authorized_by
                ? t("scheduledOutcome.note.authorized", {code: explanation.authorized_by.code})
                : t("scheduledOutcome.note.noSignaturesNeeded")
        case EScheduledOutcomeKind.RUNS_UNSIGNED:
            return t("scheduledOutcome.note.closesUnsigned")
        case EScheduledOutcomeKind.WAITING_FOR_INITIALIZATION:
            return t("scheduledOutcome.note.waitingForInitialization")
        case EScheduledOutcomeKind.REFUSED:
            // Why, then what would change it, as the server says (e.g. require the approval first).
            return t("scheduledOutcome.note.refusedWithStep", {
                reason: t(`scheduledOutcome.note.refused.${CHECK_KEY[explanation.deciding]}`),
                next: t(explanation.next_step.message_key, explanation.next_step.params ?? {}),
            })
    }
}

/** The checks table of an explanation: each question, both copies, and its verdict. */
export const OutcomeChecks: React.FC<{
    explanation: IScheduledOutcomeExplanation
    zone: string
}> = ({explanation, zone}) => {
    const {t} = useTranslation()
    const service = useTimeZoneService()
    const row = (check: IScheduledOutcomeCheck) => {
        const deciding = check.id === explanation.deciding
        return (
            <TableRow
                key={check.id}
                data-testid={`outcome-check-${check.id}`}
                selected={deciding}
                aria-current={deciding ? "true" : undefined}
            >
                <TableCell component="th" scope="row" sx={{fontWeight: deciding ? 600 : 400}}>
                    {t(`scheduledOutcome.question.${CHECK_KEY[check.id]}`)}
                    {deciding ? (
                        <Chip
                            size="small"
                            sx={{ml: 1}}
                            label={t("scheduledOutcome.why.deciding")}
                            variant="outlined"
                        />
                    ) : null}
                </TableCell>
                <TableCell>{valueText(check.current, zone, service)}</TableCell>
                <TableCell>{valueText(check.published, zone, service)}</TableCell>
                <TableCell>
                    <Box
                        component="span"
                        sx={{display: "inline-flex", alignItems: "center", gap: 0.5}}
                    >
                        {check.allows ? (
                            <CheckCircleOutlineIcon fontSize="small" color="success" aria-hidden />
                        ) : (
                            <BlockIcon fontSize="small" color="error" aria-hidden />
                        )}
                        {t(
                            check.allows
                                ? "scheduledOutcome.why.allows"
                                : "scheduledOutcome.why.blocks"
                        )}
                    </Box>
                </TableCell>
            </TableRow>
        )
    }
    return (
        <Stack spacing={1.5}>
            <Table size="small" aria-label={t("scheduledOutcome.why.checks")}>
                <TableHead>
                    <TableRow>
                        <TableCell>{t("scheduledOutcome.why.check")}</TableCell>
                        <TableCell>{t("scheduledOutcome.why.current")}</TableCell>
                        <TableCell>{t("scheduledOutcome.why.published")}</TableCell>
                        <TableCell>{t("scheduledOutcome.why.verdict")}</TableCell>
                    </TableRow>
                </TableHead>
                <TableBody>{explanation.checks.map(row)}</TableBody>
            </Table>
            <Typography variant="body2">
                <Box component="span" sx={{fontWeight: 600}}>
                    {t("scheduledOutcome.why.nextStep")}
                </Box>{" "}
                {valueText(explanation.next_step, zone, service)}
            </Typography>
        </Stack>
    )
}

/** The outcome chip of a scheduled transition ("Will run", "Will run without signatures", "Will be refused"). */
export const OutcomeChip: React.FC<{outcome: EScheduledOutcomeKind}> = ({outcome}) => {
    const {t} = useTranslation()
    return (
        <Chip
            size="small"
            color={OUTCOME_COLOR[outcome]}
            sx={WARNING_TEXT[OUTCOME_COLOR[outcome]]}
            label={t(`scheduledOutcome.chip.${OUTCOME_KEY[outcome]}`)}
        />
    )
}

/**
 * What a scheduled opening or closing will do and why (design §5c): the
 * outcome chip with its note, and "Why?", which opens the checks table, the
 * deciding check and the next step.
 */
export const ScheduledOutcome: React.FC<{
    explanation: IScheduledOutcomeExplanation
    /** The row's zone, for times in the explanation. */
    zone: string
}> = ({explanation, zone}) => {
    const {t, i18n} = useTranslation()
    const [anchor, setAnchor] = useState<HTMLElement | null>(null)
    const titleId = useId()
    return (
        <Stack spacing={0.5} sx={{alignItems: "flex-start"}} data-testid="scheduled-outcome">
            <Stack direction="row" spacing={1} sx={{alignItems: "center"}}>
                <OutcomeChip outcome={explanation.outcome} />
                <Button
                    size="small"
                    sx={{minWidth: 0, p: 0, textTransform: "none"}}
                    aria-haspopup="dialog"
                    aria-expanded={!!anchor}
                    onClick={(event) => setAnchor(event.currentTarget)}
                >
                    {t("scheduledOutcome.why.button")}
                </Button>
            </Stack>
            <Typography variant="caption" color="text.secondary">
                {outcomeNote(explanation, t)}
            </Typography>
            <Popover
                open={!!anchor}
                anchorEl={anchor}
                onClose={() => setAnchor(null)}
                anchorOrigin={{vertical: "bottom", horizontal: "left"}}
                slotProps={{
                    paper: {
                        "role": "dialog",
                        "aria-labelledby": titleId,
                        "sx": {p: 2, maxWidth: 720},
                    },
                }}
            >
                <Stack spacing={1.5}>
                    <Typography id={titleId} component="h3" sx={{fontWeight: 600}}>
                        {t(`scheduledOutcome.why.title.${OUTCOME_KEY[explanation.outcome]}`)}
                    </Typography>
                    <Typography variant="body2">{outcomeNote(explanation, t)}</Typography>
                    {explanation.authorized_by?.signers.length ? (
                        <Typography variant="body2" color="text.secondary">
                            {t("scheduledOutcome.why.signedBy", {
                                names: new Intl.ListFormat(i18n.language, {
                                    type: "conjunction",
                                }).format(explanation.authorized_by.signers),
                            })}
                        </Typography>
                    ) : null}
                    <OutcomeChecks explanation={explanation} zone={zone} />
                </Stack>
            </Popover>
        </Stack>
    )
}
