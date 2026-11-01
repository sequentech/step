// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The dialog that the check mark next to a Tracking ID opens (design board
// ReceiptDialog): the short and full Tracking ID, where the ballot stands, and
// the three checks behind it, each with what the receipt says about it.

import React, {useId} from "react"
import {Box, ButtonBase, Dialog} from "@mui/material"
import {useTranslation} from "react-i18next"
import {
    ballotProblemOf,
    ballotTagOf,
    EBallotProblem,
    EBallotStatus,
    EBallotTag,
    EStatusStep,
    EStepState,
    isAuditStatus,
} from "../../services/vitrine/ballotStatus"
import {shortBallotId} from "../../services/vitrine/ballotId"
import type {IReceiptView} from "../../services/vitrine/castReceipt"
import {receiptFileName} from "../../services/vitrine/confirmation"
import {RECEIPT_MEDIA_TYPE, saveTextFile} from "../../services/vitrine/file"
import {
    clockSeconds,
    IStatusRow,
    ITranslation,
    statusCaption,
    statusHeadline,
    statusRows,
} from "../../services/vitrine/statusRows"
import {
    BallotStatusIcon,
    CHECK_COLORS,
    CHECK_SIZES,
    CheckIcon,
    ECheckBadge,
    ECheckLine,
    ECheckMarks,
    ICheckGlyph,
} from "./BallotStatusIcon"
import {EVitrineLog} from "../../services/vitrine/types"
import {BallotTag} from "./BallotTag"
import {explorersOf} from "./explorers"
import {FindElsewhere} from "./FindElsewhere"
import {EFullTrackingIdSurface, FullTrackingId} from "./FullTrackingId"

const DIALOG_COLORS = {
    navy: "#0F054C",
    text: "#191D23",
    grey: "#757575",
    muted: "#475569",
    subtle: "#64748B",
    line: "#E7EAEE",
    mint: "#ECFDF5",
    copied: "#047857",
    amberText: "#7C2D12",
    amberFill: "#FFF7ED",
    amberLine: "#FED7AA",
    redText: "#7F1D1D",
    redFill: "#FEF2F2",
    redLine: "#FECACA",
    redStrong: "#B91C1C",
} as const

/** The board's plain outline, in place of the theme's ring inside the control. */
const focusRing = (offset: number) => ({
    outline: `2px solid ${DIALOG_COLORS.navy}`,
    outlineOffset: offset,
    boxShadow: "none",
})

const MONO = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"

const HEAD_COLORS: Partial<Record<EBallotStatus, string>> = {
    [EBallotStatus.VERIFIED]: CHECK_COLORS.BLUE,
    [EBallotStatus.LATE]: CHECK_COLORS.AMBER,
    [EBallotStatus.PROBLEM]: DIALOG_COLORS.redStrong,
    [EBallotStatus.AUDIT_FAILED]: DIALOG_COLORS.redStrong,
}

const STEP_MARKS: Record<EStatusStep, ECheckMarks> = {
    [EStatusStep.RECORD]: ECheckMarks.SINGLE,
    [EStatusStep.WITNESSES]: ECheckMarks.DOUBLE,
    [EStatusStep.ANCHORS]: ECheckMarks.DOUBLE,
}

const rowGlyph = (step: EStatusStep, state: EStepState): ICheckGlyph => {
    const marks = STEP_MARKS[step]
    switch (state) {
        case EStepState.UPCOMING:
            return {
                marks,
                line: ECheckLine.DOTTED,
                badge: ECheckBadge.NONE,
                color: CHECK_COLORS.LIGHT,
            }
        case EStepState.ACTIVE:
            return {
                marks,
                line: ECheckLine.DASHED,
                badge: ECheckBadge.NONE,
                color: CHECK_COLORS.GREY,
            }
        case EStepState.DONE:
            return {
                marks,
                line: ECheckLine.SOLID,
                badge: ECheckBadge.NONE,
                color: step === EStatusStep.ANCHORS ? CHECK_COLORS.BLUE : CHECK_COLORS.GREY,
            }
        case EStepState.LATE:
            return {
                marks,
                line: ECheckLine.SOLID,
                badge: ECheckBadge.CLOCK,
                color: CHECK_COLORS.AMBER,
            }
        case EStepState.FAILED:
            return {
                marks,
                line: ECheckLine.SOLID,
                badge: ECheckBadge.ALERT,
                color: CHECK_COLORS.RED,
            }
        default:
            return {
                marks,
                line: ECheckLine.DASHED,
                badge: ECheckBadge.NONE,
                color: CHECK_COLORS.LIGHT,
            }
    }
}

const isAhead = (state: EStepState): boolean =>
    state === EStepState.UPCOMING || state === EStepState.WAITING || state === EStepState.SKIPPED

const INFO_PATH = "M10.4 11h1.9v6.2M10.2 17.2h4.4"

const TitleBar: React.FC<{id: string; title: string; closeLabel: string; onClose: () => void}> = ({
    id,
    title,
    closeLabel,
    onClose,
}) => (
    <Box
        className="ballot-status-bar"
        sx={{display: "flex", alignItems: "center", gap: "8px", p: "16px 16px 16px 24px"}}
    >
        <svg
            className="ballot-status-info-icon"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            aria-hidden="true"
            style={{flex: "none", color: DIALOG_COLORS.navy}}
        >
            <circle
                className="ballot-status-info-disc"
                cx="12"
                cy="12"
                r="11"
                fill="currentColor"
            />
            <circle className="ballot-status-info-dot" cx="12" cy="7.4" r="1.6" fill="#fff" />
            <path
                className="ballot-status-info-mark"
                d={INFO_PATH}
                fill="none"
                stroke="#fff"
                strokeWidth="2.2"
                strokeLinecap="round"
                strokeLinejoin="round"
            />
        </svg>
        <Box
            id={id}
            component="h2"
            className="ballot-status-title"
            sx={{flex: 1, m: 0, pt: "3px", fontSize: "20px", lineHeight: 1.6, fontWeight: 700}}
        >
            {title}
        </Box>
        <ButtonBase
            className="ballot-status-close"
            aria-label={closeLabel}
            onClick={onClose}
            sx={{
                "width": 40,
                "height": 40,
                "borderRadius": "50%",
                "color": DIALOG_COLORS.navy,
                "flex": "none",
                "&.Mui-focusVisible": focusRing(1),
            }}
        >
            <svg
                className="ballot-status-close-icon"
                width="22"
                height="22"
                viewBox="0 0 24 24"
                aria-hidden="true"
            >
                <circle
                    className="ballot-status-close-disc"
                    cx="12"
                    cy="12"
                    r="11"
                    fill="currentColor"
                />
                <path
                    className="ballot-status-close-mark"
                    d="M8.5 8.5l7 7M15.5 8.5l-7 7"
                    stroke="#fff"
                    strokeWidth="2.4"
                    strokeLinecap="round"
                />
            </svg>
        </ButtonBase>
    </Box>
)

const IdBox: React.FC<{ballotId: string; tag: EBallotTag}> = ({ballotId, tag}) => {
    const {t} = useTranslation()
    const label = {fontSize: "12px", lineHeight: "16px", color: DIALOG_COLORS.grey}
    return (
        <Box
            className="ballot-status-id"
            sx={{
                bgcolor: DIALOG_COLORS.mint,
                borderRadius: "8px",
                p: "12px 14px",
                display: "flex",
                flexDirection: "column",
                gap: "8px",
            }}
        >
            <Box className="ballot-status-id-top">
                <Box
                    className="ballot-status-id-head"
                    sx={{
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                        gap: "8px",
                    }}
                >
                    <Box className="ballot-status-id-label" sx={label}>
                        {t("vitrine.ballotId.label")}
                    </Box>
                    <BallotTag tag={tag} />
                </Box>
                <Box
                    className="ballot-status-short-id"
                    sx={{
                        fontFamily: MONO,
                        fontSize: "22px",
                        lineHeight: "30px",
                        fontWeight: 700,
                        color: DIALOG_COLORS.text,
                    }}
                >
                    {shortBallotId(ballotId)}
                </Box>
            </Box>
            <FullTrackingId ballotId={ballotId} surface={EFullTrackingIdSurface.GREEN} />
        </Box>
    )
}

const Headline: React.FC<{
    status: EBallotStatus
    head: string
    sub: string
}> = ({status, head, sub}) => (
    <Box
        className="ballot-status-headline"
        sx={{display: "flex", gap: "12px", alignItems: "flex-start"}}
    >
        <Box
            className="ballot-status-headline-icon"
            sx={{mt: "1px", flex: "none", display: "flex"}}
        >
            <BallotStatusIcon status={status} size={CHECK_SIZES.DIALOG} animated={false} />
        </Box>
        <Box className="ballot-status-headline-text" sx={{minWidth: 0}}>
            <Box
                className="ballot-status-head"
                aria-live="polite"
                sx={{
                    fontSize: "18px",
                    lineHeight: "26px",
                    fontWeight: 700,
                    color: HEAD_COLORS[status] ?? DIALOG_COLORS.text,
                }}
            >
                {head}
            </Box>
            <Box
                className="ballot-status-sub"
                sx={{mt: "2px", fontSize: "14px", lineHeight: "20px", color: DIALOG_COLORS.muted}}
            >
                {sub}
            </Box>
        </Box>
    </Box>
)

const wordColor = (state: EStepState): {color: string; fontWeight: number} => {
    if (state === EStepState.LATE) return {color: CHECK_COLORS.AMBER, fontWeight: 600}
    if (state === EStepState.FAILED) return {color: DIALOG_COLORS.redStrong, fontWeight: 600}
    return {color: DIALOG_COLORS.subtle, fontWeight: 400}
}

const Steps: React.FC<{rows: IStatusRow[]; tr: (text: ITranslation) => string}> = ({rows, tr}) => (
    <Box
        component="ol"
        className="ballot-status-steps"
        sx={{
            m: 0,
            p: 0,
            listStyle: "none",
            border: `1px solid ${DIALOG_COLORS.line}`,
            borderRadius: "4px",
        }}
    >
        {rows.map((row, n) => (
            <Box
                component="li"
                key={row.step}
                className={`ballot-status-step ballot-status-step-${row.step}`}
                data-state={row.state}
                sx={{
                    display: "flex",
                    gap: "12px",
                    alignItems: "flex-start",
                    p: "12px 14px",
                    borderTop: n ? `1px solid ${DIALOG_COLORS.line}` : 0,
                }}
            >
                <Box
                    className="ballot-status-step-icon"
                    sx={{
                        width: 24,
                        height: 22,
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        flex: "none",
                    }}
                >
                    <CheckIcon glyph={rowGlyph(row.step, row.state)} size={20} animated={false} />
                </Box>
                <Box className="ballot-status-step-text" sx={{flex: 1, minWidth: 0}}>
                    <Box
                        className="ballot-status-step-title"
                        sx={{
                            fontSize: "15px",
                            lineHeight: "22px",
                            fontWeight: 700,
                            color: isAhead(row.state) ? DIALOG_COLORS.subtle : DIALOG_COLORS.text,
                        }}
                    >
                        {tr(row.title)}
                    </Box>
                    <Box
                        className="ballot-status-step-detail"
                        sx={{fontSize: "13px", lineHeight: "18px", color: DIALOG_COLORS.subtle}}
                    >
                        {tr(row.detail)}
                    </Box>
                </Box>
                <Box
                    className="ballot-status-step-word"
                    sx={{
                        fontSize: "13px",
                        lineHeight: "22px",
                        whiteSpace: "nowrap",
                        ...wordColor(row.state),
                    }}
                >
                    {row.status
                        ? tr(row.status)
                        : row.doneAt === undefined
                          ? null
                          : clockSeconds(row.doneAt)}
                </Box>
            </Box>
        ))}
    </Box>
)

const CALLOUT = {borderRadius: "4px", fontSize: "14px", lineHeight: "20px"} as const

const actionSx = {
    "minHeight": 44,
    "px": "12px",
    "py": "6px",
    "border": `1px solid ${DIALOG_COLORS.navy}`,
    "borderRadius": "4px",
    "fontSize": "16px",
    "fontWeight": 500,
    "flex": "1 1 0",
    "&.Mui-focusVisible": focusRing(2),
} as const

export interface IBallotStatusDialogProps {
    open: boolean
    onClose: () => void
    status: EBallotStatus
    /** The receipt this status comes from; none before the ballot is cast. */
    view?: IReceiptView
    /** The tag, when the status doesn't tell it: a ballot received at review
     * climbs the same checks as a cast one, and is still not cast. */
    tag?: EBallotTag
    ballotId: string
    electionName?: string
    /** When the ballot was cast, or audited, as the voter reads it. */
    castAt?: string
    /** When this page saw each step done, in milliseconds. */
    times?: Partial<Record<EStatusStep, number>>
    /** Whether this device keeps the receipt and checks it on each visit. */
    kept?: boolean
    /** The ballot in the ballot tracker. */
    tracker?: React.AnchorHTMLAttributes<HTMLAnchorElement>
    /** Audits the ballot, at review: the dialog offers it, next to OK, for a
     * ballot that is not cast yet. */
    onAudit?: () => void
}

export const BallotStatusDialog: React.FC<IBallotStatusDialogProps> = ({
    open,
    onClose,
    status,
    view,
    tag,
    ballotId,
    electionName,
    castAt,
    times,
    kept = false,
    tracker,
    onAudit,
}) => {
    const {t} = useTranslation()
    const titleId = useId()
    const notCast = status === EBallotStatus.NOT_CAST
    const problem = status === EBallotStatus.PROBLEM ? ballotProblemOf(view) : undefined
    const input = {status, problem, result: view?.result, policy: view?.policy, times}
    const headline = statusHeadline(input)
    const rows = statusRows(input)
    const tr = ({key, values}: ITranslation): string => t(key, values)
    const chain = view?.chain
    // The ballot is in the record once its first check has an answer.
    const inRecord = status !== EBallotStatus.NOT_CAST && status !== EBallotStatus.CHECKING
    const explorers = inRecord ? explorersOf(view?.policy) : []
    const entry = view?.result?.entry
    const reviewing = notCast || tag === EBallotTag.NOT_CAST
    const canAudit =
        !!onAudit && reviewing && status !== EBallotStatus.PROBLEM && !isAuditStatus(status)
    const save = () => {
        if (chain) saveTextFile(receiptFileName(ballotId), chain, RECEIPT_MEDIA_TYPE)
    }
    const when = isAuditStatus(status) ? "vitrine.status.audited" : "vitrine.status.cast"
    const meta = [electionName, castAt ? t(when, {when: castAt}) : undefined]
        .filter(Boolean)
        .join(" · ")

    return (
        <Dialog
            open={open}
            onClose={onClose}
            aria-labelledby={titleId}
            // A portal's events bubble to its React parents, and the ballot
            // list's card starts the vote when it is clicked.
            onClick={(event) => event.stopPropagation()}
            maxWidth={false}
            classes={{paper: "ballot-status-dialog"}}
            slotProps={{
                paper: {
                    sx: {
                        width: {xs: "calc(100vw - 64px)", sm: 520},
                        border: 0,
                        borderRadius: "4px",
                        color: DIALOG_COLORS.text,
                    },
                },
            }}
        >
            <TitleBar
                id={titleId}
                title={t("vitrine.status.title")}
                closeLabel={t("a11y.closeDialog")}
                onClose={onClose}
            />
            <Box
                className="ballot-status-body"
                sx={{display: "flex", flexDirection: "column", gap: "16px", p: "0 24px 4px"}}
            >
                {meta ? (
                    <Box
                        className="ballot-status-meta"
                        sx={{fontSize: "14px", lineHeight: "20px", color: DIALOG_COLORS.grey}}
                    >
                        {meta}
                    </Box>
                ) : null}
                <IdBox ballotId={ballotId} tag={tag ?? ballotTagOf(status)} />
                <Headline status={status} head={tr(headline.head)} sub={tr(headline.sub)} />
                <Steps rows={rows} tr={tr} />
                {status === EBallotStatus.LATE ? (
                    <Box
                        className="ballot-status-late"
                        sx={{
                            ...CALLOUT,
                            bgcolor: DIALOG_COLORS.amberFill,
                            border: `1px solid ${DIALOG_COLORS.amberLine}`,
                            p: "12px 14px",
                            color: DIALOG_COLORS.amberText,
                        }}
                    >
                        {t("vitrine.status.callout.late")}{" "}
                        {t(
                            kept
                                ? "vitrine.status.callout.lateKept"
                                : "vitrine.status.callout.lateTracker"
                        )}
                    </Box>
                ) : null}
                {(problem === EBallotProblem.MISMATCH || status === EBallotStatus.AUDIT_FAILED) &&
                chain ? (
                    <Box
                        className="ballot-status-problem"
                        sx={{
                            ...CALLOUT,
                            bgcolor: DIALOG_COLORS.redFill,
                            border: `1px solid ${DIALOG_COLORS.redLine}`,
                            p: "14px",
                            color: DIALOG_COLORS.redText,
                            display: "flex",
                            flexDirection: "column",
                            gap: "12px",
                        }}
                    >
                        <Box className="ballot-status-problem-text">
                            {t("vitrine.status.callout.evidence")}
                        </Box>
                        <ButtonBase
                            className="ballot-status-save-evidence"
                            onClick={save}
                            sx={{
                                ...actionSx,
                                alignSelf: "flex-start",
                                flex: "none",
                                px: "16px",
                                bgcolor: DIALOG_COLORS.redStrong,
                                borderColor: DIALOG_COLORS.redStrong,
                                color: "#fff",
                            }}
                        >
                            {t("vitrine.status.callout.saveEvidence")}
                        </ButtonBase>
                    </Box>
                ) : null}
                <FindElsewhere
                    explorers={explorers}
                    policyId={view?.policy?.id}
                    log={entry?.log ?? EVitrineLog.BALLOTS}
                    ballotId={ballotId}
                />
                <Box
                    className="ballot-status-caption"
                    sx={{
                        display: "flex",
                        gap: "8px",
                        alignItems: "flex-start",
                        fontSize: "13px",
                        lineHeight: "18px",
                        color: DIALOG_COLORS.subtle,
                    }}
                >
                    <svg
                        className="ballot-status-caption-icon"
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
                        <circle className="ballot-status-caption-lens" cx="11" cy="11" r="6.5" />
                        <path className="ballot-status-caption-handle" d="M16 16l4.5 4.5" />
                    </svg>
                    <span className="ballot-status-caption-text">
                        {statusCaption(status, kept).map(tr).join(" ")}
                        {!notCast && tracker ? (
                            <>
                                {" "}
                                <Box
                                    component="a"
                                    className="ballot-status-tracker-link"
                                    {...tracker}
                                    sx={{color: DIALOG_COLORS.navy, fontWeight: 500}}
                                >
                                    {t("vitrine.status.tracker")}
                                </Box>
                            </>
                        ) : null}
                    </span>
                </Box>
                {canAudit ? (
                    <Box
                        className="ballot-status-audit-note"
                        sx={{
                            p: "12px 14px",
                            borderRadius: "4px",
                            bgcolor: "#F8FAFC",
                            fontSize: "14px",
                            lineHeight: "20px",
                            color: "#334155",
                        }}
                    >
                        {t("vitrine.status.auditNote")}
                    </Box>
                ) : null}
            </Box>
            <Box
                className="ballot-status-actions"
                sx={{
                    display: "flex",
                    flexDirection: {xs: "column", sm: "row"},
                    gap: "8px",
                    p: "16px 24px 20px",
                }}
            >
                {canAudit ? (
                    <ButtonBase
                        className="ballot-status-audit"
                        onClick={onAudit}
                        sx={{
                            ...actionSx,
                            bgcolor: "#fff",
                            borderColor: DIALOG_COLORS.redStrong,
                            color: DIALOG_COLORS.redStrong,
                            gap: "8px",
                        }}
                    >
                        <svg
                            className="ballot-status-audit-icon"
                            width="16"
                            height="16"
                            viewBox="0 0 24 24"
                            fill="currentColor"
                            aria-hidden="true"
                        >
                            <path
                                className="ballot-status-audit-flame"
                                d="M13.5 2.5c.4 3-1.2 4.6-2.8 6.2C9 10.4 7.5 12 7.5 14.6a4.5 4.5 0 0 0 9 0c0-1.4-.5-2.6-1.2-3.6.1 1.3-.4 2.3-1.4 2.8.6-3.3-.2-6.8-.4-11.3z"
                            />
                        </svg>
                        {t("vitrine.status.audit")}
                    </ButtonBase>
                ) : null}
                {notCast ? null : (
                    <ButtonBase
                        className="ballot-status-save"
                        onClick={save}
                        disabled={!chain}
                        sx={{
                            ...actionSx,
                            "bgcolor": "#fff",
                            "color": DIALOG_COLORS.navy,
                            "&.Mui-disabled": {
                                color: DIALOG_COLORS.subtle,
                                borderColor: DIALOG_COLORS.line,
                            },
                        }}
                    >
                        {t("vitrine.status.save")}
                    </ButtonBase>
                )}
                <ButtonBase
                    className="ballot-status-ok"
                    onClick={onClose}
                    sx={{...actionSx, bgcolor: DIALOG_COLORS.navy, color: "#fff"}}
                >
                    {t("vitrine.status.ok")}
                </ButtonBase>
            </Box>
        </Dialog>
    )
}
