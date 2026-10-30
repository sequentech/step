// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The result of a receipt check, as the design board ReceiptDialog shows it
// inline: the ballot's status, the three steps behind it, what to do when a
// step is late or doesn't match, and who checked. Its words, and the colours
// of its steps, follow the status the result gives (services/vitrine/
// ballotStatus.ts).

import React, {useState} from "react"
import Box from "@mui/material/Box"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {theme} from "@sequentech/ui-essentials"
import type {ChainResult, Policy} from "@vitrine/verify"
import {
    ballotProblemOf,
    ballotStatusOf,
    EBallotProblem,
    EBallotStatus,
    EStatusStep,
    EStepState,
    IStatusFacts,
    statusFacts,
    statusSteps,
    STATUS_STEPS,
} from "../../services/vitrine/ballotStatus"
import type {TStepTimes} from "../../screens/hooks/useStepTimes"
import {BallotStatusIcon, CHECK_COLORS, CHECK_SIZES, CheckIcon, stepGlyph} from "./BallotStatusIcon"

const COLORS = {
    TEXT: theme.palette.customGrey.contrastText,
    SUB: "#475569",
    MUTED: theme.palette.customGrey.dark,
    BORDER: theme.palette.customGrey.light,
    PROBLEM: "#B91C1C",
    LATE_BACKGROUND: "#FFF7ED",
    LATE_BORDER: "#FED7AA",
    LATE_TEXT: "#7C2D12",
    PROBLEM_BACKGROUND: "#FEF2F2",
    PROBLEM_BORDER: theme.palette.red.light,
    PROBLEM_TEXT: "#7F1D1D",
    PROBLEM_LINK: "#991B1B",
}

const Rows = styled("ol")`
    margin: 0;
    padding: 0;
    list-style: none;
    border: 1px solid ${COLORS.BORDER};
    border-radius: 4px;
`

const Row = styled("li")`
    display: flex;
    gap: 12px;
    align-items: flex-start;
    padding: 12px 14px;

    & + & {
        border-top: 1px solid ${COLORS.BORDER};
    }
`

const Callout = styled(Box)`
    padding: 12px 14px;
    border-radius: 4px;
    font-size: 14px;
    line-height: 20px;
`

const CalloutButton = styled("button")`
    min-height: 44px;
    padding: 6px 16px;
    border: 1px solid ${COLORS.PROBLEM};
    border-radius: 4px;
    background: ${COLORS.PROBLEM};
    color: #fff;
    font-family: inherit;
    font-size: 16px;
    font-weight: 500;
    cursor: pointer;

    &:focus-visible {
        outline: 2px solid ${COLORS.PROBLEM_TEXT};
        outline-offset: 2px;
    }
`

const CalloutLink = styled(CalloutButton)`
    padding: 6px 12px;
    border: 0;
    background: transparent;
    color: ${COLORS.PROBLEM_LINK};
    text-decoration: underline;
    text-underline-offset: 3px;
`

const HEADLINE_COLORS: Partial<Record<EBallotStatus, string>> = {
    [EBallotStatus.VERIFIED]: CHECK_COLORS.BLUE,
    [EBallotStatus.LATE]: CHECK_COLORS.AMBER,
    [EBallotStatus.PROBLEM]: COLORS.PROBLEM,
}

const CLOCK_FORMAT: Intl.DateTimeFormatOptions = {
    hourCycle: "h23",
    hour: "2-digit",
    minute: "2-digit",
}

/** A time of day, as `10:42` or, with seconds, `10:42:08`. */
export const clockTime = (seconds: number, lang: string, withSeconds: boolean): string =>
    new Date(seconds * 1000).toLocaleTimeString(lang, {
        ...CLOCK_FORMAT,
        ...(withSeconds ? {second: "2-digit"} : {}),
    })

const STEP_TITLES: Record<EStatusStep, string> = {
    [EStatusStep.RECORD]: "receiptScreen.status.rows.record",
    [EStatusStep.WITNESSES]: "receiptScreen.status.rows.witnesses",
    [EStatusStep.ANCHORS]: "receiptScreen.status.rows.anchors",
}

interface IHeadline {
    head: string
    sub: string
    color?: string
}

interface IRowText {
    detail: string
    status: string
    titleColor?: string
    statusColor?: string
    statusWeight?: number
}

export interface IReceiptStatusProps {
    /** The check's result; none until the first round is done. */
    result?: ChainResult
    policy: Policy
    /** True while the full round is still to come after the log round. */
    fullDue: boolean
    /** When each step was first seen done, POSIX seconds. */
    times: TStepTimes
    lang: string
    /** The user asked to save the evidence for a mismatch. */
    onEvidence: () => void
}

export const ReceiptStatus: React.FC<IReceiptStatusProps> = ({
    result,
    policy,
    fullDue,
    times,
    lang,
    onEvidence,
}) => {
    const {t} = useTranslation()
    const [reportOpen, setReportOpen] = useState(false)
    const status = ballotStatusOf(result)
    const problem = ballotProblemOf(result)
    const steps = statusSteps(result)
    const facts: IStatusFacts | undefined = result ? statusFacts(result, policy) : undefined
    const key = (path: string) => `receiptScreen.status.${path}`

    const headline = (): IHeadline => {
        const words = (name: string, sub: string, options = {}) => ({
            head: t(key(`${name}.head`)),
            sub: t(key(`${name}.${sub}`), options),
            color: HEADLINE_COLORS[status] ?? COLORS.TEXT,
        })
        switch (status) {
            case EBallotStatus.WITNESSED:
                return words("witnessed", fullDue ? "sub" : "subLater")
            case EBallotStatus.LATE:
                return words(
                    "late",
                    steps[EStatusStep.ANCHORS] === EStepState.LATE ? "sub" : "subOther"
                )
            case EBallotStatus.PROBLEM:
                if (problem === EBallotProblem.DAMAGED) return words("damaged", "sub")
                return facts?.index !== undefined && steps[EStatusStep.RECORD] === EStepState.FAILED
                    ? words("problem", "sub", {index: facts.index})
                    : words("problem", "subOther")
            default:
                return words(status, "sub")
        }
    }

    const rowText = (step: EStatusStep): IRowText => {
        const state = steps[step]
        const time = times[step]
        const done = time === undefined ? "" : clockTime(time, lang, true)
        const wait = t(key(step === EStatusStep.ANCHORS ? "rows.within10" : "rows.withinSeconds"))
        switch (state) {
            case EStepState.DONE:
                return {detail: doneDetail(step), status: facts?.unused.includes(step) ? "" : done}
            case EStepState.ACTIVE:
                if (step !== EStatusStep.ANCHORS) {
                    return {detail: t(key("rows.checking")), status: t(key("rows.statusChecking"))}
                }
                return fullDue
                    ? {detail: t(key("rows.anchorsActive")), status: t(key("rows.within30"))}
                    : {detail: t(key("rows.anchorsPending")), status: wait}
            case EStepState.LATE:
                return {
                    detail:
                        step === EStatusStep.ANCHORS && facts?.anchorsDue !== undefined
                            ? t(key("rows.lateAnchors"), {
                                  time: clockTime(facts.anchorsDue, lang, false),
                              })
                            : t(key("rows.lateOther")),
                    status: t(key("rows.statusLate")),
                    statusColor: CHECK_COLORS.AMBER,
                    statusWeight: 600,
                }
            case EStepState.FAILED:
                return {
                    detail:
                        step === EStatusStep.RECORD && facts?.index !== undefined
                            ? t(key("rows.recordFailed"), {index: facts.index})
                            : t(key("rows.failed")),
                    status: t(key("rows.statusMismatch")),
                    statusColor: COLORS.PROBLEM,
                    statusWeight: 600,
                }
            case EStepState.DAMAGED:
                return {
                    detail: t(key("rows.damaged")),
                    status: t(key("rows.statusDamaged")),
                    statusColor: COLORS.PROBLEM,
                    statusWeight: 600,
                }
            case EStepState.SKIPPED:
                return {
                    detail: t(
                        key(
                            problem === EBallotProblem.DAMAGED
                                ? "rows.skippedDamaged"
                                : "rows.skipped"
                        )
                    ),
                    status: "—",
                    titleColor: COLORS.MUTED,
                }
            default:
                return {
                    detail: t(key("rows.waiting")),
                    status: wait,
                    titleColor: COLORS.MUTED,
                }
        }
    }

    const doneDetail = (step: EStatusStep): string => {
        if (!facts) return ""
        if (facts.unused.includes(step)) return t(key("rows.unused"))
        switch (step) {
            case EStatusStep.RECORD:
                return facts.index === undefined
                    ? t(key("rows.recordDoneNoIndex"))
                    : t(key("rows.recordDone"), {index: facts.index})
            case EStatusStep.WITNESSES:
                return facts.signed !== undefined &&
                    facts.witnesses !== undefined &&
                    facts.needed !== undefined
                    ? t(key("rows.witnessesDone"), {
                          signed: facts.signed,
                          total: facts.witnesses,
                          needed: facts.needed,
                      })
                    : t(key("rows.witnessesDoneAny"))
            default: {
                const layer = facts.browser === "waict" || facts.browser === "bootstrap"
                const standIn = facts.browser === "bootstrap"
                return `${t(key("rows.hubs"), {count: facts.hubs})}${
                    layer ? t(key("rows.browser")) : ""
                }${standIn ? t(key("rows.standIn")) : ""}.`
            }
        }
    }

    const {head, sub, color} = headline()
    const caption =
        status === EBallotStatus.CHECKING
            ? "captionChecking"
            : problem === EBallotProblem.MISMATCH
              ? "captionProblem"
              : "captionChecked"

    return (
        <Box
            data-testid="receipt-status"
            data-status={status}
            sx={{display: "flex", flexDirection: "column", gap: "16px"}}
        >
            <Box sx={{display: "flex", gap: "12px", alignItems: "flex-start"}}>
                <Box sx={{marginTop: "1px", display: "flex"}}>
                    <BallotStatusIcon status={status} size={CHECK_SIZES.STATUS} />
                </Box>
                <Box sx={{minWidth: 0}}>
                    <Box
                        aria-live="polite"
                        data-testid="status-headline"
                        sx={{fontSize: 18, lineHeight: "26px", fontWeight: 700, color}}
                    >
                        {head}
                    </Box>
                    <Box
                        sx={{marginTop: "2px", fontSize: 14, lineHeight: "20px", color: COLORS.SUB}}
                    >
                        {sub}
                    </Box>
                </Box>
            </Box>

            <Rows>
                {STATUS_STEPS.map((step) => {
                    const text = rowText(step)
                    return (
                        <Row key={step} data-testid={`status-row-${step}`} data-state={steps[step]}>
                            <Box
                                sx={{
                                    width: 24,
                                    height: 22,
                                    display: "flex",
                                    alignItems: "center",
                                    justifyContent: "center",
                                    flex: "none",
                                }}
                            >
                                <CheckIcon
                                    glyph={stepGlyph(step, steps[step])}
                                    size={CHECK_SIZES.ROW}
                                />
                            </Box>
                            <Box sx={{flex: 1, minWidth: 0}}>
                                <Box
                                    sx={{
                                        fontSize: 15,
                                        lineHeight: "22px",
                                        fontWeight: 700,
                                        color: text.titleColor ?? COLORS.TEXT,
                                    }}
                                >
                                    {t(STEP_TITLES[step])}
                                </Box>
                                <Box sx={{fontSize: 13, lineHeight: "18px", color: COLORS.MUTED}}>
                                    {text.detail}
                                </Box>
                            </Box>
                            <Box
                                sx={{
                                    fontSize: 13,
                                    lineHeight: "22px",
                                    whiteSpace: "nowrap",
                                    color: text.statusColor ?? COLORS.MUTED,
                                    fontWeight: text.statusWeight ?? 400,
                                }}
                            >
                                {text.status}
                            </Box>
                        </Row>
                    )
                })}
            </Rows>

            {status === EBallotStatus.LATE ? (
                <Callout
                    data-testid="late-callout"
                    sx={{
                        background: COLORS.LATE_BACKGROUND,
                        border: `1px solid ${COLORS.LATE_BORDER}`,
                        color: COLORS.LATE_TEXT,
                    }}
                >
                    {t("receiptScreen.result.advice.late")}
                </Callout>
            ) : null}
            {problem === EBallotProblem.MISMATCH ? (
                <Callout
                    data-testid="problem-callout"
                    sx={{
                        padding: "14px",
                        background: COLORS.PROBLEM_BACKGROUND,
                        border: `1px solid ${COLORS.PROBLEM_BORDER}`,
                        color: COLORS.PROBLEM_TEXT,
                        display: "flex",
                        flexDirection: "column",
                        gap: "12px",
                    }}
                >
                    <Box>{t(key("evidence"))}</Box>
                    <Box sx={{display: "flex", gap: "8px", flexWrap: "wrap"}}>
                        <CalloutButton onClick={onEvidence} data-testid="save-evidence">
                            {t(key("saveEvidence"))}
                        </CalloutButton>
                        <CalloutLink
                            onClick={() => setReportOpen((open) => !open)}
                            aria-expanded={reportOpen}
                            data-testid="how-to-report"
                        >
                            {t(key("howToReport"))}
                        </CalloutLink>
                    </Box>
                    {reportOpen ? (
                        <Box data-testid="report-advice">
                            {t("receiptScreen.result.advice.failed")}
                        </Box>
                    ) : null}
                </Callout>
            ) : null}

            <Box
                sx={{
                    display: "flex",
                    gap: "8px",
                    alignItems: "flex-start",
                    fontSize: 13,
                    lineHeight: "18px",
                    color: COLORS.MUTED,
                }}
            >
                <svg
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    aria-hidden="true"
                    style={{flex: "none", marginTop: 1}}
                >
                    <circle cx="11" cy="11" r="6.5" />
                    <path d="M16 16l4.5 4.5" />
                </svg>
                <span>{t(key(caption))}</span>
            </Box>
        </Box>
    )
}
