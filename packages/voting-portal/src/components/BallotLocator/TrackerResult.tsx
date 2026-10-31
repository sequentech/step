// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The ballot tracker's result for an event with transparency (design boards
// TrackerResult and PhoneTracker): the ballot's IDs and its receipt on one
// side, on the other where the ballot stands in the record, step by step.

import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import {Box, ButtonBase, Typography} from "@mui/material"
import {keyframes, styled} from "@mui/material/styles"
import {useCopyFeedback} from "../../hooks/useCopyFeedback"
import {
    ballotIdGroups,
    groupBallotId,
    NARROW_GROUPS_PER_LINE,
    shortBallotId,
} from "../../services/vitrine/ballotId"
import {EAuditOutcome, TAudit} from "../../services/vitrine/ballotAudit"
import {EBallotStatus} from "../../services/vitrine/ballotStatus"
import {
    ETrackerState,
    ETrackerStep,
    ETrackerView,
    ITrackerReceipt,
    ITrackerStep,
    recordFileName,
    ringProgress,
    TRACKER_STEPS,
    trackedResult,
    trackerStatusOf,
    trackerSteps,
    trackerViewOf,
} from "../../services/vitrine/ballotTracker"
import {receiptFileName} from "../../services/vitrine/confirmation"
import {RECEIPT_MEDIA_TYPE, saveTextFile} from "../../services/vitrine/file"
import {openVerifier} from "../../services/vitrine/handoff"
import {BallotStatusIcon, CHECK_COLORS} from "../Vitrine/BallotStatusIcon"
import {RecordViewer} from "../Vitrine/RecordViewer"
import {EStatusTone, StatusTag} from "../Vitrine/StatusTag"
import {BallotIdLines} from "./BallotIdLines"
import {MONO, PALETTE, PHONE, TABLET} from "./trackerStyle"

const RING_RADII = [30, 38, 46, 54] as const

const STEP_COLORS: Record<ETrackerStep, string> = {
    [ETrackerStep.RECORD]: PALETTE.navy,
    [ETrackerStep.WITNESSES]: PALETTE.navy,
    [ETrackerStep.HUBS]: CHECK_COLORS.BLUE,
    [ETrackerStep.BROWSER]: CHECK_COLORS.BLUE,
}

const stepColor = (step: ETrackerStep, state: ETrackerState | undefined): string => {
    if (state === ETrackerState.LATE) return CHECK_COLORS.AMBER
    if (state === ETrackerState.FAILED) return CHECK_COLORS.RED
    return STEP_COLORS[step]
}

const Arc = styled("circle")`
    transition:
        stroke-dashoffset 0.9s cubic-bezier(0.2, 0.8, 0.2, 1),
        stroke 0.3s;

    @media (prefers-reduced-motion: reduce) {
        transition: none;
    }
`

interface ISealProps {
    steps: ITrackerStep[]
    status: EBallotStatus
    label: string
}

/** One ring for each step, filled as the step is checked, around the check
 * mark of the ballot's status. */
const Seal: React.FC<ISealProps> = ({steps, status, label}) => (
    <Box
        className="tracker-seal"
        sx={{
            position: "relative",
            width: 140,
            height: 140,
            [PHONE]: {width: 96, height: 96},
        }}
    >
        <svg
            className="tracker-seal-rings"
            width="100%"
            height="100%"
            viewBox="0 0 140 140"
            role="img"
            aria-label={label}
            style={{transform: "rotate(-90deg)"}}
        >
            {RING_RADII.map((r) => (
                <circle
                    className="tracker-seal-track"
                    key={`track-${r}`}
                    cx="70"
                    cy="70"
                    r={r}
                    fill="none"
                    stroke={PALETTE.track}
                    strokeWidth="4.5"
                />
            ))}
            {TRACKER_STEPS.map((step, n) => {
                const r = RING_RADII[n]
                const circumference = +(2 * Math.PI * r).toFixed(2)
                const state = steps.find((s) => s.step === step)?.state
                const progress = ringProgress(state)
                return (
                    <Arc
                        key={`ring-${step}`}
                        className={`tracker-ring tracker-ring-${step}`}
                        cx="70"
                        cy="70"
                        r={r}
                        fill="none"
                        stroke={stepColor(step, state)}
                        opacity={progress > 0 ? 1 : 0}
                        strokeWidth="4.5"
                        strokeLinecap="round"
                        strokeDasharray={`${circumference} 1000`}
                        strokeDashoffset={+(circumference * (1 - progress)).toFixed(2)}
                    />
                )
            })}
        </svg>
        <Box
            className="tracker-seal-mark"
            aria-hidden="true"
            sx={{
                position: "absolute",
                left: "50%",
                top: "50%",
                width: 52,
                height: 52,
                margin: "-26px 0 0 -26px",
                borderRadius: "50%",
                backgroundColor: "#fff",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                boxShadow: "0 1px 2px rgba(15,5,76,.08)",
                [PHONE]: {
                    "width": 40,
                    "height": 40,
                    "margin": "-20px 0 0 -20px",
                    "& svg": {width: 24, height: 24},
                },
            }}
        >
            <BallotStatusIcon status={status} size={30} />
        </Box>
    </Box>
)

const pulse = keyframes`
    from {
        opacity: 0.6;
        transform: scale(0.7);
    }
    to {
        opacity: 0;
        transform: scale(1.2);
    }
`

const Pulse = styled("span")`
    position: absolute;
    inset: -6px;
    border-radius: 50%;
    border: 2px solid ${PALETTE.navy};
    animation: ${pulse} 1.4s ease-out infinite;

    @media (prefers-reduced-motion: reduce) {
        animation: none;
        opacity: 0.4;
    }
`

const ICON_PROPS = {
    "width": 14,
    "height": 14,
    "viewBox": "0 0 24 24",
    "fill": "none",
    "strokeWidth": 3,
    "strokeLinecap": "round",
    "strokeLinejoin": "round",
    "focusable": false,
    "aria-hidden": true,
} as const

interface IMarker {
    background: string
    border: string
    glyph?: "check" | "clock" | "alert"
    glyphColor: string
    pulsing?: boolean
}

const markerOf = (item: ITrackerStep): IMarker => {
    const color = stepColor(item.step, item.state)
    switch (item.state) {
        case ETrackerState.DONE:
        case ETrackerState.FAILED:
            return {
                background: color,
                border: color,
                glyph: item.state === ETrackerState.DONE ? "check" : "alert",
                glyphColor: "#fff",
            }
        case ETrackerState.LATE:
            return {background: color, border: color, glyph: "clock", glyphColor: "#fff"}
        case ETrackerState.ACTIVE:
            return {
                background: "#fff",
                border: PALETTE.navy,
                glyphColor: "#fff",
                pulsing: true,
            }
        case ETrackerState.SCHEDULED:
            return {
                background: "#fff",
                border: color,
                glyph: "clock",
                glyphColor: color,
            }
        default:
            return {background: "#fff", border: PALETTE.pending, glyphColor: "#fff"}
    }
}

const MarkerGlyph: React.FC<{glyph: IMarker["glyph"]; color: string}> = ({glyph, color}) => {
    if (glyph === "check") {
        return (
            <svg className="tracker-marker-icon" {...ICON_PROPS} stroke={color}>
                <path className="tracker-marker-icon-path" d="M5 12.5l4.5 4.5L19 7.5" />
            </svg>
        )
    }
    if (glyph === "clock") {
        return (
            <svg className="tracker-marker-icon" {...ICON_PROPS} stroke={color}>
                <path className="tracker-marker-icon-path" d="M12 6.5V12l3.5 2" />
            </svg>
        )
    }
    if (glyph === "alert") {
        return (
            <svg className="tracker-marker-icon" {...ICON_PROPS} stroke={color}>
                <path className="tracker-marker-icon-path" d="M12 6.5v6.5M12 17.5v.1" />
            </svg>
        )
    }
    return null
}

interface IStepRowProps {
    item: ITrackerStep
    last: boolean
    formatTime: (milliseconds: number) => string
    language: string
}

const listOf = (names: string[], language: string): string => {
    try {
        return new Intl.ListFormat(language, {style: "long", type: "conjunction"}).format(names)
    } catch {
        return names.join(", ")
    }
}

const StepRow: React.FC<IStepRowProps> = ({item, last, formatTime, language}) => {
    const {t} = useTranslation()
    const {step, state} = item
    const key = `ballotLocator.result.steps.${step}`
    const marker = markerOf(item)
    const at = (seconds: number | undefined) =>
        seconds === undefined ? "" : formatTime(seconds * 1000)
    const failedRecord = state === ETrackerState.FAILED && step === ETrackerStep.RECORD
    const hubCount = item.hubs?.length ?? 0
    let title =
        step !== ETrackerStep.HUBS
            ? t(`${key}.title`)
            : hubCount > 0
              ? t(`${key}.title`, {count: hubCount})
              : t(`${key}.titleAny`)
    let detail = ""
    let time = ""
    let timeColor: string = PALETTE.quiet
    let timeWeight = 400
    switch (state) {
        case ETrackerState.DONE:
            if (step === ETrackerStep.HUBS) {
                detail = t(`${key}.done`, {hubs: listOf(item.hubs ?? [], language)})
            } else if (step !== ETrackerStep.RECORD || item.index !== undefined) {
                detail = t(`${key}.done`, {index: item.index})
            }
            time = at(item.at)
            break
        case ETrackerState.ACTIVE:
            detail = t("ballotLocator.result.steps.checking")
            break
        case ETrackerState.SCHEDULED:
            detail = t("ballotLocator.result.steps.scheduled", {count: item.minutes ?? 1})
            time =
                item.due === undefined
                    ? ""
                    : t("ballotLocator.result.steps.by", {time: at(item.due)})
            timeColor = CHECK_COLORS.BLUE
            break
        case ETrackerState.LATE:
            if (item.due !== undefined) {
                detail = t(
                    step === ETrackerStep.HUBS
                        ? "ballotLocator.result.steps.lateHubs"
                        : "ballotLocator.result.steps.lateOther",
                    {time: at(item.due)}
                )
            }
            time = t("ballotLocator.result.steps.late")
            timeColor = CHECK_COLORS.AMBER
            timeWeight = 600
            break
        case ETrackerState.FAILED:
            title = failedRecord ? t("ballotLocator.result.status.problem.head") : title
            detail = t(failedRecord ? `${key}.failed` : "ballotLocator.result.steps.failedStep")
            time = t(failedRecord ? `${key}.missing` : "ballotLocator.result.steps.failedTime")
            timeColor = PALETTE.redInk
            timeWeight = 600
            break
        case ETrackerState.SKIPPED:
            detail = t("ballotLocator.result.steps.skipped")
            break
        default:
            detail = t("ballotLocator.result.steps.waiting")
    }
    const quiet = state === ETrackerState.WAITING || state === ETrackerState.SKIPPED
    const lineColor = state === ETrackerState.DONE ? stepColor(step, state) : PALETTE.border

    return (
        <Box
            component="li"
            className={`tracker-step tracker-step-${step} tracker-step-${state}`}
            sx={{display: "flex", gap: "14px"}}
        >
            <Box
                className="tracker-step-row"
                sx={{
                    display: "flex",
                    flexDirection: "column",
                    alignItems: "center",
                    width: 24,
                    flex: "none",
                }}
            >
                <Box
                    className="tracker-step-marker"
                    component="span"
                    sx={{
                        position: "relative",
                        width: 24,
                        height: 24,
                        borderRadius: "50%",
                        border: `2px solid ${marker.border}`,
                        backgroundColor: marker.background,
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        flex: "none",
                        transition: "background .3s, border-color .3s",
                    }}
                >
                    <MarkerGlyph glyph={marker.glyph} color={marker.glyphColor} />
                    {marker.pulsing ? <Pulse className="tracker-step-pulse" /> : null}
                </Box>
                {last ? null : (
                    <Box
                        className="tracker-step-line"
                        component="span"
                        sx={{
                            width: 2,
                            flex: 1,
                            minHeight: 16,
                            margin: "4px 0",
                            backgroundColor: lineColor,
                            transition: "background .6s",
                        }}
                    />
                )}
            </Box>
            <Box className="tracker-step-body" sx={{flex: 1, minWidth: 0, paddingBottom: "20px"}}>
                <Box
                    className="tracker-step-heading"
                    sx={{display: "flex", justifyContent: "space-between", gap: "12px"}}
                >
                    <Typography
                        component="span"
                        className="tracker-step-title"
                        sx={{
                            margin: 0,
                            fontSize: 16,
                            lineHeight: "24px",
                            fontWeight: 700,
                            color: quiet ? PALETTE.quiet : PALETTE.text,
                        }}
                    >
                        {title}
                    </Typography>
                    <Typography
                        component="span"
                        className="tracker-step-time"
                        sx={{
                            margin: 0,
                            fontSize: 13,
                            whiteSpace: "nowrap",
                            color: timeColor,
                            fontWeight: timeWeight,
                            alignSelf: "baseline",
                        }}
                    >
                        {time}
                    </Typography>
                </Box>
                <Box
                    className="tracker-step-detail"
                    sx={{fontSize: 14, lineHeight: "20px", color: PALETTE.muted, marginTop: "2px"}}
                >
                    {detail}
                </Box>
                {item.witnesses && state === ETrackerState.DONE ? (
                    <Box
                        className="tracker-witnesses"
                        sx={{
                            display: "flex",
                            gap: "6px",
                            alignItems: "center",
                            marginTop: "8px",
                            fontSize: 13,
                            color: PALETTE.quiet,
                        }}
                    >
                        {Array.from({length: item.witnesses.total}, (_, n) => (
                            <Box
                                className="tracker-witness"
                                key={n}
                                component="span"
                                aria-hidden="true"
                                sx={{
                                    width: 22,
                                    height: 22,
                                    borderRadius: "50%",
                                    backgroundColor:
                                        n < item.witnesses!.signed ? PALETTE.green : PALETTE.track,
                                    display: "flex",
                                    alignItems: "center",
                                    justifyContent: "center",
                                }}
                            >
                                {n < item.witnesses!.signed ? (
                                    <svg
                                        className="tracker-witness-icon"
                                        {...ICON_PROPS}
                                        width={12}
                                        height={12}
                                        strokeWidth={3.2}
                                        stroke="#fff"
                                    >
                                        <path
                                            className="tracker-witness-icon-path"
                                            d="M5 12.5l4.5 4.5L19 7.5"
                                        />
                                    </svg>
                                ) : null}
                            </Box>
                        ))}
                        <span className="tracker-witness-count" style={{marginLeft: 4}}>
                            {item.witnesses.needed === undefined
                                ? t("ballotLocator.result.steps.witnesses.countOf", {
                                      signed: item.witnesses.signed,
                                      total: item.witnesses.total,
                                  })
                                : t("ballotLocator.result.steps.witnesses.count", {
                                      signed: item.witnesses.signed,
                                      total: item.witnesses.total,
                                      needed: item.witnesses.needed,
                                  })}
                        </span>
                    </Box>
                ) : null}
            </Box>
        </Box>
    )
}

const CopyButton = styled(ButtonBase)`
    position: absolute;
    right: 8px;
    top: 8px;
    height: 28px;
    width: 28px;
    border-radius: 4px;
    background: #fff;
    color: ${PALETTE.text};
    opacity: 0;
    transition: opacity 0.15s;

    &:focus-visible,
    &.copied {
        opacity: 1;
    }

    &.copied {
        background: ${PALETTE.greenTint};
        color: ${PALETTE.green};
    }

    @media (hover: none) {
        opacity: 1;
    }
`

const FullId: React.FC<{ballotId: string}> = ({ballotId}) => {
    const {t} = useTranslation()
    const {copied, copy} = useCopyFeedback()

    return (
        <Box
            className="tracker-full-id-box"
            sx={{
                "position": "relative",
                "width": "100%",
                "padding": "12px 14px",
                "borderRadius": "8px",
                "backgroundColor": "#fff",
                "border": `1px solid ${PALETTE.border}`,
                "textAlign": "left",
                "&:hover .tracker-copy-button": {opacity: 1},
                [PHONE]: {padding: "10px 12px"},
            }}
        >
            <Box className="tracker-full-id-label" sx={KEY_SX} style={{marginBottom: 4}}>
                {t("ballotLocator.result.fullId")}
            </Box>
            <Box
                className="tracker-full-id"
                sx={{font: `400 14px/22px ${MONO}`, color: PALETTE.navy}}
            >
                <BallotIdLines ballotId={ballotId} perLine={NARROW_GROUPS_PER_LINE} />
            </Box>
            <CopyButton
                className={`tracker-copy-button${copied ? " copied" : ""}`}
                onClick={() => copy(groupBallotId(ballotId))}
                aria-label={t(copied ? "ballotLocator.result.copied" : "ballotLocator.result.copy")}
            >
                {copied ? (
                    <svg
                        className="tracker-copied-icon"
                        {...ICON_PROPS}
                        width={16}
                        height={16}
                        strokeWidth={2}
                        stroke="currentColor"
                    >
                        <path className="tracker-copied-icon-path" d="M4.5 12.5l4.75 4.75L19.5 7" />
                    </svg>
                ) : (
                    <svg
                        className="tracker-copy-icon"
                        {...ICON_PROPS}
                        width={16}
                        height={16}
                        strokeWidth={2}
                        stroke="currentColor"
                    >
                        <rect
                            className="tracker-copy-icon-front"
                            x="8"
                            y="8"
                            width="12"
                            height="12"
                            rx="2"
                        />
                        <path
                            className="tracker-copy-icon-back"
                            d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"
                        />
                    </svg>
                )}
            </CopyButton>
        </Box>
    )
}

const KEY_SX = {
    fontSize: 12,
    fontWeight: 500,
    letterSpacing: ".08em",
    textTransform: "uppercase",
    color: PALETTE.quiet,
} as const

/** A hash as the receipt names it: its first three groups and its last. */
const abbreviated = (hex: string): string => {
    const groups = hex.replace(/\s+/g, "").toLowerCase().match(/.{1,4}/g) ?? []
    return groups.length > 4
        ? `${groups.slice(0, 3).join(" ")} ··· ${groups.at(-1)}`
        : groups.join(" ")
}

const ActionButton = styled(ButtonBase)`
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 6px;
    border-radius: 4px;
    font-size: 14px;
    font-weight: 500;
    color: ${PALETTE.navy};

    &:hover {
        text-decoration: underline;
        text-underline-offset: 2px;
    }
`

const Callout = styled(Box)`
    padding: 14px 16px;
    border-radius: 4px;
    font-size: 14px;
    line-height: 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
`

const CALLOUT_COLORS = {
    late: {backgroundColor: "#FFF7ED", border: "1px solid #FED7AA", color: "#7C2D12"},
    problem: {backgroundColor: "#FEF2F2", border: "1px solid #FECACA", color: "#7F1D1D"},
} as const

const HEAD_COLORS: Partial<Record<EBallotStatus, string>> = {
    [EBallotStatus.VERIFIED]: CHECK_COLORS.BLUE,
    [EBallotStatus.LATE]: CHECK_COLORS.AMBER,
    [EBallotStatus.PROBLEM]: PALETTE.redInk,
}

export interface ITrackerResultProps {
    /** The found ballot's full ID. */
    ballotId: string
    /** The record's content: the ballot as the public record holds it. */
    content: string
    /** The audit of the record, when it holds an audited ballot. */
    audit?: TAudit
    tracked: ITrackerReceipt
    verifierUrl: string
    language: string
    formatTime: (milliseconds: number) => string
}

interface IKindTag {
    labelKey: string
    tone: EStatusTone
}

/** What the tag next to the short ID says the ballot is: cast, or audited,
 * and how the audit went. */
const kindTagOf = (audit: TAudit | undefined): IKindTag => {
    switch (audit?.outcome) {
        case EAuditOutcome.VERIFIED:
            return {labelKey: "vitrine.status.tag.auditable", tone: EStatusTone.SUCCESS}
        case EAuditOutcome.MISMATCH:
            return {labelKey: "ballotLocator.result.tag.auditFailed", tone: EStatusTone.DANGER}
        case EAuditOutcome.NOT_VERIFIABLE:
            return {labelKey: "vitrine.status.tag.auditable", tone: EStatusTone.NEUTRAL}
        default:
            return {labelKey: "vitrine.status.tag.cast", tone: EStatusTone.SUCCESS}
    }
}

/** The receipt side by side with the status block. */
export const TrackerResult: React.FC<ITrackerResultProps> = ({
    ballotId,
    content,
    audit,
    tracked,
    verifierUrl,
    language,
    formatTime,
}) => {
    const {t} = useTranslation()
    const [reporting, setReporting] = useState(false)
    const {view} = tracked
    const shown = trackerViewOf(tracked)
    const result = trackedResult(view)
    const status = trackerStatusOf(view)
    const steps = trackerSteps(view)
    const chain = view?.chain
    const hubs = steps.find((s) => s.step === ETrackerStep.HUBS)
    const browser = steps.find((s) => s.step === ETrackerStep.BROWSER)
    const recordFailed = steps[0]?.state === ETrackerState.FAILED
    const anchored =
        status === EBallotStatus.WITNESSED &&
        hubs?.state === ETrackerState.DONE &&
        browser?.state === ETrackerState.SCHEDULED
    const statusKey = status === EBallotStatus.PROBLEM && !recordFailed ? "otherProblem" : status
    const head = t(`ballotLocator.result.status.${statusKey}.head`)
    const sub = anchored
        ? t("ballotLocator.result.status.witnessed.subAnchored", {count: browser?.minutes ?? 1})
        : t(`ballotLocator.result.status.${statusKey}.sub`)
    const caption =
        status === EBallotStatus.VERIFIED
            ? "captionVerified"
            : status === EBallotStatus.PROBLEM
              ? "captionProblem"
              : "caption"
    const entry = result?.entry?.leafHash
    const saveReceipt = chain
        ? () => saveTextFile(receiptFileName(ballotId), chain, RECEIPT_MEDIA_TYPE)
        : undefined
    const blockShown = shown === ETrackerView.STATUS || shown === ETrackerView.CHECKING
    const kindTag = kindTagOf(audit)

    return (
        <Box
            component="section"
            className="tracker-receipt"
            aria-label={t("ballotLocator.result.receipt")}
            sx={{
                marginTop: "24px",
                display: "grid",
                gridTemplateColumns: "360px minmax(0, 1fr)",
                border: `1px solid ${PALETTE.border}`,
                borderRadius: "4px",
                overflow: "hidden",
                [TABLET]: {gridTemplateColumns: "minmax(0, 1fr)"},
            }}
        >
            <Box
                className="tracker-ids"
                sx={{
                    backgroundColor: PALETTE.tint,
                    padding: "32px 32px 28px",
                    display: "flex",
                    flexDirection: "column",
                    alignItems: "center",
                    textAlign: "center",
                    gap: "16px",
                    borderRight: `1px solid ${PALETTE.border}`,
                    [TABLET]: {borderRight: 0, borderBottom: `1px solid ${PALETTE.border}`},
                    [PHONE]: {padding: "18px 16px 16px", gap: "10px"},
                }}
            >
                {blockShown ? (
                    <Seal
                        steps={steps}
                        status={status}
                        label={t("ballotLocator.result.sealLabel", {status: head})}
                    />
                ) : null}
                <Box className="tracker-id">
                    <Box className="tracker-id-label" sx={KEY_SX}>
                        {t("a11y.ballotIdLabel")}
                    </Box>
                    <Box
                        className="tracker-short-id"
                        sx={{
                            font: `700 30px/38px ${MONO}`,
                            color: PALETTE.navy,
                            [PHONE]: {font: `700 24px/30px ${MONO}`},
                        }}
                    >
                        {shortBallotId(ballotId)}
                    </Box>
                    <Box
                        className="tracker-tags"
                        sx={{
                            marginTop: "8px",
                            display: "flex",
                            justifyContent: "center",
                            gap: "6px",
                        }}
                    >
                        <StatusTag label={t(kindTag.labelKey)} tone={kindTag.tone} />
                        <StatusTag label={t("ballotLocator.result.tag.found")} />
                    </Box>
                </Box>
                <Typography
                    className="tracker-id-help"
                    sx={{
                        margin: 0,
                        fontSize: 14,
                        lineHeight: 1.45,
                        color: PALETTE.muted,
                        maxWidth: 270,
                    }}
                >
                    {t("ballotLocator.result.idComment")}
                </Typography>
                <FullId ballotId={ballotId} />
                {entry ? (
                    <Box
                        className="tracker-record-entry-line"
                        sx={{fontSize: 13, color: PALETTE.quiet}}
                    >
                        {t("ballotLocator.result.recordEntry")}{" "}
                        <Box
                            component="code"
                            className="tracker-entry"
                            sx={{font: `400 13px ${MONO}`, color: PALETTE.muted}}
                        >
                            {abbreviated(entry)}
                        </Box>
                    </Box>
                ) : null}
                {saveReceipt ? (
                    <ActionButton className="tracker-save-button" onClick={saveReceipt}>
                        <svg
                            className="tracker-save-icon"
                            {...ICON_PROPS}
                            width={16}
                            height={16}
                            strokeWidth={2}
                            stroke="currentColor"
                        >
                            <path
                                className="tracker-save-icon-path"
                                d="M12 4v11M7 10l5 5 5-5M5 20h14"
                            />
                        </svg>
                        {t("ballotLocator.result.save")}
                    </ActionButton>
                ) : null}
            </Box>

            <Box
                className="tracker-status"
                sx={{
                    padding: "28px 32px 24px",
                    display: "flex",
                    flexDirection: "column",
                    gap: "20px",
                    [PHONE]: {padding: "16px", gap: "14px"},
                }}
            >
                {blockShown ? (
                    <>
                        <Box className="tracker-status-block" aria-live="polite">
                            <Box
                                className="tracker-status-head"
                                sx={{
                                    fontSize: 22,
                                    lineHeight: "28px",
                                    fontWeight: 700,
                                    color: HEAD_COLORS[status] ?? PALETTE.text,
                                }}
                            >
                                {head}
                            </Box>
                            <Box
                                className="tracker-status-sub"
                                sx={{
                                    fontSize: 14,
                                    lineHeight: "20px",
                                    color: PALETTE.muted,
                                    mt: "2px",
                                }}
                            >
                                {sub}
                            </Box>
                        </Box>
                        <Box
                            component="ol"
                            className="tracker-steps"
                            sx={{listStyle: "none", margin: 0, padding: 0}}
                        >
                            {steps.map((item, n) => (
                                <StepRow
                                    key={item.step}
                                    item={item}
                                    last={n === steps.length - 1}
                                    formatTime={formatTime}
                                    language={language}
                                />
                            ))}
                        </Box>
                        {status === EBallotStatus.LATE ? (
                            <Callout className="tracker-late" sx={CALLOUT_COLORS.late}>
                                {t("ballotLocator.result.late")}
                            </Callout>
                        ) : null}
                        {status === EBallotStatus.PROBLEM ? (
                            <Callout
                                className="tracker-problem"
                                role="alert"
                                sx={CALLOUT_COLORS.problem}
                            >
                                <span className="tracker-problem-text">
                                    {t(
                                        recordFailed
                                            ? "ballotLocator.result.problem"
                                            : "ballotLocator.result.problemOther"
                                    )}
                                </span>
                                <Box
                                    className="tracker-problem-actions"
                                    sx={{display: "flex", gap: "8px", flexWrap: "wrap"}}
                                >
                                    {saveReceipt ? (
                                        <ButtonBase
                                            className="tracker-save-evidence-button"
                                            onClick={saveReceipt}
                                            sx={{
                                                minHeight: 44,
                                                padding: "6px 12px",
                                                borderRadius: "4px",
                                                border: `1px solid ${PALETTE.redInk}`,
                                                backgroundColor: PALETTE.redInk,
                                                color: "#fff",
                                                fontSize: 16,
                                                fontWeight: 500,
                                            }}
                                        >
                                            {t("ballotLocator.result.saveEvidence")}
                                        </ButtonBase>
                                    ) : null}
                                    <ButtonBase
                                        className="tracker-how-to-report-button"
                                        aria-expanded={reporting}
                                        onClick={() => setReporting(!reporting)}
                                        sx={{
                                            minHeight: 44,
                                            padding: "6px 12px",
                                            color: "#991B1B",
                                            fontSize: 16,
                                            fontWeight: 500,
                                            textDecoration: "underline",
                                            textUnderlineOffset: "3px",
                                        }}
                                    >
                                        {t("ballotLocator.result.howToReport")}
                                    </ButtonBase>
                                </Box>
                                {reporting ? (
                                    <span className="tracker-how-to-report-text">
                                        {t("ballotLocator.result.howToReportText")}
                                    </span>
                                ) : null}
                            </Callout>
                        ) : null}
                        <Box
                            className="tracker-caption"
                            sx={{
                                display: "flex",
                                alignItems: "center",
                                justifyContent: "space-between",
                                gap: "12px",
                                padding: "12px 14px",
                                borderRadius: "4px",
                                backgroundColor: PALETTE.tint,
                                fontSize: 14,
                                color: PALETTE.muted,
                                [PHONE]: {flexDirection: "column", alignItems: "flex-start"},
                            }}
                        >
                            <span
                                className="tracker-caption-text"
                                style={{display: "flex", alignItems: "center", gap: 8}}
                            >
                                <svg
                                    className="tracker-caption-icon"
                                    {...ICON_PROPS}
                                    width={16}
                                    height={16}
                                    strokeWidth={2}
                                    stroke="currentColor"
                                >
                                    <circle
                                        className="tracker-caption-icon-lens"
                                        cx="11"
                                        cy="11"
                                        r="6.5"
                                    />
                                    <path
                                        className="tracker-caption-icon-handle"
                                        d="M16 16l4.5 4.5"
                                    />
                                </svg>
                                {t(`ballotLocator.result.${caption}`)}
                            </span>
                            {chain && view?.own ? (
                                <ButtonBase
                                    className="tracker-check-button"
                                    role="link"
                                    onClick={() => void openVerifier(verifierUrl, view, chain)}
                                    sx={{
                                        color: PALETTE.navy,
                                        fontSize: 14,
                                        fontWeight: 500,
                                        display: "inline-flex",
                                        alignItems: "center",
                                        gap: "4px",
                                        whiteSpace: "nowrap",
                                    }}
                                >
                                    {t("ballotLocator.result.check")}
                                    <svg
                                        className="tracker-check-icon"
                                        {...ICON_PROPS}
                                        width={14}
                                        height={14}
                                        strokeWidth={2}
                                        stroke="currentColor"
                                    >
                                        <path
                                            className="tracker-check-icon-path"
                                            d="M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5"
                                        />
                                    </svg>
                                </ButtonBase>
                            ) : null}
                        </Box>
                    </>
                ) : (
                    <Box
                        className="tracker-note"
                        role="status"
                        sx={{
                            padding: "14px 16px",
                            borderRadius: "4px",
                            backgroundColor: PALETTE.tint,
                            fontSize: 14,
                            lineHeight: "20px",
                            color: PALETTE.muted,
                        }}
                    >
                        {t(
                            shown === ETrackerView.NO_RECEIPT
                                ? "ballotLocator.result.noReceipt"
                                : "ballotLocator.result.receiptPending"
                        )}
                    </Box>
                )}
                <RecordViewer
                    className="ballot-content"
                    title={t("ballotLocator.result.record.title")}
                    subtitle={t("ballotLocator.result.record.subtitle")}
                    filename={recordFileName(ballotId)}
                    content={content}
                />
            </Box>
        </Box>
    )
}
