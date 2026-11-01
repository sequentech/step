// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The check at the end of a ballot audit, on a hub's page (design board
// AuditCompare, where=hub; the voting portal's
// components/BallotLocator/AuditCompare.tsx): this browser has decoded the
// audited ballot from the public record, and only the voter knows what they
// chose, so the voter compares. Both answers look the same, so the page
// leans neither way. A match sends the voter back to the voting site; a
// mismatch tells them not to vote on that device and offers the evidence to
// report. Nothing is sent from here, and the hub has nothing to print.

import React, {useEffect, useId, useRef, useState} from "react"
import {useTranslation} from "react-i18next"
import Box from "@mui/material/Box"
import ButtonBase from "@mui/material/ButtonBase"
import {EContactSource, IAuthorityContact, mailtoOf} from "../../services/vitrine/auditEvidence"
import {PALETTE} from "./hubStyle"

/** Below this width the steps stack, the answers fill the row and the panels
 * lose their indent. */
const NARROW = 560

enum EAnswer {
    NONE = "none",
    MATCH = "match",
    MISMATCH = "mismatch",
}

enum EStep {
    DONE = "done",
    CURRENT = "current",
    PROBLEM = "problem",
}

const COLORS = {
    rail: "#D5DAE5",
    hover: "#F1F5F9",
    answerHover: "#F1F3FA",
    greenBorder: "#A7F3D0",
    greenInk: "#065F46",
    redTint: "#FEF2F2",
    redBorder: "#FECACA",
    redDeep: "#7F1D1D",
} as const

const KEYS = "auditedScreen.compare"

const FOCUS = {outline: `2px solid ${PALETTE.navy}`, outlineOffset: "2px"} as const

const ANSWER_SX = {
    "display": "inline-flex",
    "alignItems": "center",
    "justifyContent": "center",
    "gap": "6px",
    "minHeight": 44,
    "p": "0 16px",
    "border": `1px solid ${PALETTE.navy}`,
    "borderRadius": "4px",
    "backgroundColor": "#fff",
    "fontSize": 15,
    "fontWeight": 500,
    "color": PALETTE.navy,
    "textDecoration": "none",
    "&:hover": {backgroundColor: COLORS.answerHover},
    "&:focus-visible": FOCUS,
} as const

/** The way back to the voting site, once the ballot matches. */
const BACK_SX = {
    ...ANSWER_SX,
    "backgroundColor": PALETTE.navy,
    "color": "#fff",
    "&:hover": {backgroundColor: PALETTE.navy, opacity: 0.92},
} as const

const LINK_SX = {
    "p": "4px 2px",
    "fontSize": 14,
    "color": "inherit",
    "textDecoration": "underline",
    "textUnderlineOffset": "3px",
    "&:focus-visible": FOCUS,
} as const

const BADGE_SX = {flex: "none"} as const

const GoodBadge: React.FC = () => (
    <svg
        className="audit-compare-badge audit-compare-badge-good"
        width="24"
        height="24"
        viewBox="0 0 24 24"
        aria-hidden="true"
        style={BADGE_SX}
    >
        <circle className="audit-compare-badge-disc" cx="12" cy="12" r="12" fill={PALETTE.green} />
        <path
            className="audit-compare-badge-mark"
            d="M6.5 12.5l3.5 3.5 7.5-8"
            fill="none"
            stroke="#fff"
            strokeWidth="2.6"
            strokeLinecap="round"
            strokeLinejoin="round"
        />
    </svg>
)

const BadBadge: React.FC = () => (
    <svg
        className="audit-compare-badge audit-compare-badge-bad"
        width="24"
        height="24"
        viewBox="0 0 24 24"
        aria-hidden="true"
        style={BADGE_SX}
    >
        <circle
            className="audit-compare-badge-disc"
            cx="12"
            cy="12"
            r="12"
            fill={PALETTE.redBorder}
        />
        <path
            className="audit-compare-badge-mark"
            d="M12 6.5v7M12 17.3v.1"
            fill="none"
            stroke="#fff"
            strokeWidth="2.6"
            strokeLinecap="round"
        />
    </svg>
)

const Tick: React.FC = () => (
    <svg
        className="audit-compare-tick"
        width="13"
        height="13"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="3.4"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
    >
        <path className="audit-compare-tick-path" d="M4.5 12.5l4.75 4.75L19.5 7" />
    </svg>
)

const STEP_KEYS = ["encrypted", "decoded", "compare"] as const

const stepCircleSx = (state: EStep) =>
    ({
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        width: 26,
        height: 26,
        border: "1.5px solid",
        borderRadius: "50%",
        fontSize: 13,
        fontWeight: 700,
        ...(state === EStep.DONE
            ? {backgroundColor: PALETTE.navy, borderColor: PALETTE.navy, color: "#fff"}
            : state === EStep.PROBLEM
              ? {
                    backgroundColor: PALETTE.redBorder,
                    borderColor: PALETTE.redBorder,
                    color: "#fff",
                }
              : {backgroundColor: "#fff", borderColor: PALETTE.navy, color: PALETTE.navy}),
    }) as const

const StepCircle: React.FC<{state: EStep; number: number}> = ({state, number}) => (
    <Box component="span" className="audit-compare-step-mark" sx={stepCircleSx(state)}>
        {state === EStep.DONE ? <Tick /> : state === EStep.PROBLEM ? "!" : number}
    </Box>
)

const FAQ_KEYS = ["why", "catch", "spent", "differ"] as const

export interface IAuditCompareProps {
    /** The audited ballot's full Tracking ID. */
    ballotId: string
    /** The voting site, where the voter goes back to vote once the ballot
     * matches; without it, the button goes back in this tab's history. */
    backHref?: string
    /** Saves the evidence file on this device. */
    onSaveEvidence: () => void
    /** Who to report a mismatch to, when the page knows. */
    contact?: IAuthorityContact
    /** The choices the ballot decodes to, for the voter to compare. */
    children: React.ReactNode
}

const detailsOf = (contact: IAuthorityContact): string =>
    [contact.name, contact.email, contact.phone, contact.url].filter(Boolean).join(" · ")

export const AuditCompare: React.FC<IAuditCompareProps> = ({
    ballotId,
    backHref,
    onSaveEvidence,
    contact,
    children,
}) => {
    const {t} = useTranslation()
    const titleId = useId()
    const questionId = useId()
    const faqId = useId()
    const [answer, setAnswer] = useState(EAnswer.NONE)
    const [how, setHow] = useState(false)
    const panel = useRef<HTMLDivElement>(null)
    const firstAnswer = useRef<HTMLButtonElement>(null)
    const changed = useRef(false)

    // The answers give way to the panel, and the panel to the answers: keep
    // the keyboard where the page is.
    useEffect(() => {
        if (answer !== EAnswer.NONE) panel.current?.focus()
        else if (changed.current) firstAnswer.current?.focus()
    }, [answer])

    const change = () => {
        changed.current = true
        setAnswer(EAnswer.NONE)
    }

    const stepState = (n: number): EStep => {
        if (n < STEP_KEYS.length - 1) return EStep.DONE
        if (answer === EAnswer.MATCH) return EStep.DONE
        return answer === EAnswer.MISMATCH ? EStep.PROBLEM : EStep.CURRENT
    }

    const email = contact
        ? mailtoOf(
              contact,
              t(`${KEYS}.mismatch.emailSubject`),
              t(`${KEYS}.mismatch.emailBody`, {id: ballotId})
          )
        : undefined
    const details = contact ? detailsOf(contact) : ""

    return (
        <Box
            component="section"
            className="audit-compare"
            aria-labelledby={titleId}
            sx={{
                marginTop: "24px",
                border: `1px solid ${PALETTE.border}`,
                borderRadius: "4px",
                backgroundColor: "#fff",
                containerType: "inline-size",
                [`@container (max-width: ${NARROW}px)`]: {
                    "& .audit-compare-in": {p: "18px 16px"},
                    "& .audit-compare-steps": {gridTemplateColumns: "minmax(0, 1fr)"},
                    "& .audit-compare-step": {
                        "display": "grid",
                        "gridTemplateColumns": "26px minmax(0, 1fr)",
                        "columnGap": "12px",
                        "p": "0 0 14px",
                        "&:not(:last-child)::after": {
                            left: "13px",
                            right: "auto",
                            top: "32px",
                            bottom: "4px",
                            width: "1px",
                            height: "auto",
                        },
                    },
                    "& .audit-compare-step-title": {marginTop: "3px"},
                    "& .audit-compare-step-text": {gridColumn: 2},
                    "& .audit-compare-question": {
                        flexDirection: "column",
                        alignItems: "stretch",
                        p: "14px",
                    },
                    "& .audit-compare-answers": {flexDirection: "column"},
                    "& .audit-compare-panel-text, & .audit-compare-contact, & .audit-compare-actions":
                        {marginLeft: 0},
                    "& .audit-compare-faq": {gridTemplateColumns: "minmax(0, 1fr)"},
                },
            }}
        >
            <Box className="audit-compare-in" sx={{p: "24px"}}>
                <Box
                    className="audit-compare-top"
                    sx={{
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                        gap: "12px",
                        minHeight: 32,
                    }}
                >
                    <Box
                        className="audit-compare-kicker"
                        sx={{
                            fontSize: 12,
                            lineHeight: "16px",
                            fontWeight: 600,
                            letterSpacing: ".08em",
                            textTransform: "uppercase",
                            color: PALETTE.quiet,
                        }}
                    >
                        {t(`${KEYS}.kicker`)}
                    </Box>
                </Box>
                <Box
                    component="h2"
                    id={titleId}
                    className="audit-compare-title"
                    sx={{m: "2px 0 0", fontSize: 20, lineHeight: "28px", fontWeight: 700}}
                >
                    {t(`${KEYS}.title`)}
                </Box>
                <Box
                    component="p"
                    className="audit-compare-purpose"
                    sx={{
                        m: "4px 0 0",
                        maxWidth: 640,
                        fontSize: 15,
                        lineHeight: "22px",
                        color: PALETTE.muted,
                    }}
                >
                    {t(`${KEYS}.note`)}
                </Box>

                <Box
                    component="ol"
                    className="audit-compare-steps"
                    aria-label={t(`${KEYS}.steps.label`)}
                    sx={{
                        display: "grid",
                        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
                        m: "20px 0 0",
                        p: "16px 0 0",
                        borderTop: `1px solid ${PALETTE.track}`,
                        listStyle: "none",
                    }}
                >
                    {STEP_KEYS.map((key, n) => {
                        const state = stepState(n)
                        return (
                            <Box
                                component="li"
                                key={key}
                                className={`audit-compare-step audit-compare-step-${key}`}
                                data-state={state}
                                aria-current={state === EStep.CURRENT ? "step" : undefined}
                                sx={{
                                    "position": "relative",
                                    "pr": "20px",
                                    "&:not(:last-child)::after": {
                                        content: '""',
                                        position: "absolute",
                                        left: "38px",
                                        right: "10px",
                                        top: "13px",
                                        height: "1px",
                                        backgroundColor: COLORS.rail,
                                    },
                                }}
                            >
                                <StepCircle state={state} number={n + 1} />
                                <Box
                                    className="audit-compare-step-title"
                                    sx={{
                                        mt: "10px",
                                        fontSize: 14,
                                        lineHeight: "20px",
                                        fontWeight: 600,
                                    }}
                                >
                                    {t(`${KEYS}.steps.${key}.title`)}
                                </Box>
                                <Box
                                    className="audit-compare-step-text"
                                    sx={{
                                        mt: "2px",
                                        fontSize: 13,
                                        lineHeight: "19px",
                                        color: PALETTE.quiet,
                                    }}
                                >
                                    {t(`${KEYS}.steps.${key}.text`)}
                                </Box>
                            </Box>
                        )
                    })}
                </Box>

                <Box
                    className="audit-compare-choices"
                    role="group"
                    aria-label={t(`${KEYS}.choices`)}
                    sx={{mt: "22px"}}
                >
                    {children}
                </Box>

                {answer === EAnswer.NONE ? (
                    <Box
                        className="audit-compare-question"
                        sx={{
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "space-between",
                            flexWrap: "wrap",
                            gap: "12px 20px",
                            mt: "22px",
                            p: "16px 18px",
                            borderRadius: "8px",
                            backgroundColor: PALETTE.tint,
                        }}
                    >
                        <Box
                            id={questionId}
                            className="audit-compare-question-text"
                            sx={{
                                fontSize: 17,
                                lineHeight: "24px",
                                fontWeight: 700,
                                color: PALETTE.navy,
                            }}
                        >
                            {t(`${KEYS}.question`)}
                        </Box>
                        <Box
                            className="audit-compare-answers"
                            role="group"
                            aria-labelledby={questionId}
                            sx={{display: "flex", flexWrap: "wrap", gap: "8px"}}
                        >
                            <ButtonBase
                                ref={firstAnswer}
                                className="audit-compare-yes"
                                onClick={() => setAnswer(EAnswer.MATCH)}
                                sx={ANSWER_SX}
                            >
                                {t(`${KEYS}.yes`)}
                            </ButtonBase>
                            <ButtonBase
                                className="audit-compare-no"
                                onClick={() => setAnswer(EAnswer.MISMATCH)}
                                sx={ANSWER_SX}
                            >
                                {t(`${KEYS}.no`)}
                            </ButtonBase>
                        </Box>
                    </Box>
                ) : null}

                {answer === EAnswer.MATCH ? (
                    <Box
                        ref={panel}
                        tabIndex={-1}
                        role="status"
                        className="audit-compare-match"
                        sx={{
                            mt: "22px",
                            p: "16px 18px",
                            border: `1px solid ${COLORS.greenBorder}`,
                            borderRadius: "8px",
                            backgroundColor: PALETTE.greenTint,
                            color: COLORS.greenInk,
                            outline: "none",
                        }}
                    >
                        <Box
                            component="h3"
                            className="audit-compare-panel-title"
                            sx={{
                                display: "flex",
                                alignItems: "center",
                                gap: "10px",
                                m: 0,
                                fontSize: 17,
                                lineHeight: "24px",
                                fontWeight: 700,
                            }}
                        >
                            <GoodBadge />
                            {t(`${KEYS}.match.title`)}
                        </Box>
                        <Box
                            component="p"
                            className="audit-compare-panel-text"
                            sx={{m: "6px 0 0 34px", fontSize: 14, lineHeight: "21px"}}
                        >
                            {t(`${KEYS}.match.text`)}
                        </Box>
                        <Box
                            className="audit-compare-actions"
                            sx={{
                                display: "flex",
                                alignItems: "center",
                                flexWrap: "wrap",
                                gap: "8px 12px",
                                m: "12px 0 0 34px",
                            }}
                        >
                            {backHref ? (
                                <ButtonBase
                                    component="a"
                                    href={backHref}
                                    className="audit-compare-back"
                                    sx={BACK_SX}
                                >
                                    {t(`${KEYS}.match.back`)}
                                </ButtonBase>
                            ) : (
                                <ButtonBase
                                    className="audit-compare-back"
                                    onClick={() => window.history.back()}
                                    sx={BACK_SX}
                                >
                                    {t(`${KEYS}.match.back`)}
                                </ButtonBase>
                            )}
                            <ButtonBase
                                className="audit-compare-change"
                                onClick={change}
                                sx={LINK_SX}
                            >
                                {t(`${KEYS}.change`)}
                            </ButtonBase>
                        </Box>
                    </Box>
                ) : null}

                {answer === EAnswer.MISMATCH ? (
                    <Box
                        ref={panel}
                        tabIndex={-1}
                        role="alert"
                        className="audit-compare-mismatch"
                        sx={{
                            mt: "22px",
                            p: "16px 18px",
                            border: `1px solid ${COLORS.redBorder}`,
                            borderRadius: "8px",
                            backgroundColor: COLORS.redTint,
                            color: COLORS.redDeep,
                            outline: "none",
                        }}
                    >
                        <Box
                            component="h3"
                            className="audit-compare-panel-title"
                            sx={{
                                display: "flex",
                                alignItems: "center",
                                gap: "10px",
                                m: 0,
                                fontSize: 17,
                                lineHeight: "24px",
                                fontWeight: 700,
                                color: PALETTE.redText,
                            }}
                        >
                            <BadBadge />
                            {t(`${KEYS}.mismatch.title`)}
                        </Box>
                        <Box
                            component="p"
                            className="audit-compare-panel-text"
                            sx={{m: "6px 0 0 34px", fontSize: 14, lineHeight: "21px"}}
                        >
                            {t(`${KEYS}.mismatch.text`)}
                        </Box>
                        {details ? (
                            <Box
                                className="audit-compare-contact"
                                sx={{
                                    m: "10px 0 0 34px",
                                    p: "10px 12px",
                                    border: `1px solid ${COLORS.redBorder}`,
                                    borderRadius: "6px",
                                    backgroundColor: "#fff",
                                    fontSize: 14,
                                    lineHeight: "20px",
                                    color: PALETTE.text,
                                }}
                            >
                                <Box
                                    component="b"
                                    className="audit-compare-report"
                                    sx={{fontWeight: 600}}
                                >
                                    {t(`${KEYS}.mismatch.report`)}
                                </Box>
                                <Box className="audit-compare-contact-details">{details}</Box>
                                <Box
                                    className="audit-compare-contact-source"
                                    sx={{fontSize: 12, color: PALETTE.quiet}}
                                >
                                    {t(
                                        contact?.source === EContactSource.POLICY
                                            ? `${KEYS}.mismatch.fromPolicy`
                                            : `${KEYS}.mismatch.fromSettings`
                                    )}
                                </Box>
                            </Box>
                        ) : (
                            <Box
                                component="p"
                                className="audit-compare-panel-text audit-compare-report"
                                sx={{m: "10px 0 0 34px", fontSize: 14, lineHeight: "21px"}}
                            >
                                {t(`${KEYS}.mismatch.reportNoContact`)}
                            </Box>
                        )}
                        <Box
                            component="p"
                            className="audit-compare-panel-text"
                            sx={{m: "10px 0 0 34px", fontSize: 14, lineHeight: "21px"}}
                        >
                            {t(`${KEYS}.mismatch.then`)}
                        </Box>
                        <Box
                            className="audit-compare-actions"
                            sx={{
                                display: "flex",
                                alignItems: "center",
                                flexWrap: "wrap",
                                gap: "8px 12px",
                                m: "12px 0 0 34px",
                            }}
                        >
                            <ButtonBase
                                className="audit-compare-save"
                                onClick={onSaveEvidence}
                                sx={{
                                    ...ANSWER_SX,
                                    "backgroundColor": PALETTE.redInk,
                                    "borderColor": PALETTE.redInk,
                                    "color": "#fff",
                                    "&:hover": {backgroundColor: PALETTE.redInk, opacity: 0.92},
                                }}
                            >
                                {t(`${KEYS}.mismatch.save`)}
                            </ButtonBase>
                            {email ? (
                                <Box
                                    component="a"
                                    href={email}
                                    className="audit-compare-email"
                                    sx={{
                                        ...ANSWER_SX,
                                        borderColor: PALETTE.redInk,
                                        color: PALETTE.redText,
                                    }}
                                >
                                    {t(`${KEYS}.mismatch.email`)}
                                </Box>
                            ) : null}
                            <ButtonBase
                                className="audit-compare-change"
                                onClick={change}
                                sx={LINK_SX}
                            >
                                {t(`${KEYS}.change`)}
                            </ButtonBase>
                        </Box>
                    </Box>
                ) : null}

                <ButtonBase
                    className="audit-compare-how"
                    aria-expanded={how}
                    aria-controls={how ? faqId : undefined}
                    onClick={() => setHow(!how)}
                    sx={{
                        "display": "inline-flex",
                        "alignItems": "center",
                        "gap": "6px",
                        "minHeight": 36,
                        "mt": "14px",
                        "ml": "-6px",
                        "p": "0 6px",
                        "borderRadius": "4px",
                        "fontSize": 14,
                        "fontWeight": 500,
                        "color": PALETTE.navy,
                        "&:hover": {backgroundColor: COLORS.hover},
                        "&.Mui-focusVisible": FOCUS,
                    }}
                >
                    {t(`${KEYS}.how`)}
                    <svg
                        className="audit-compare-how-chevron"
                        width="14"
                        height="14"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="2.4"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        aria-hidden="true"
                        style={{transform: how ? "rotate(180deg)" : "none"}}
                    >
                        <path className="audit-compare-how-chevron-path" d="M6 9l6 6 6-6" />
                    </svg>
                </ButtonBase>
                {how ? (
                    <Box
                        id={faqId}
                        className="audit-compare-faq"
                        sx={{
                            display: "grid",
                            gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
                            gap: "14px 28px",
                            m: "8px 0 0",
                            p: "16px 0 0",
                            borderTop: `1px solid ${PALETTE.track}`,
                        }}
                    >
                        {FAQ_KEYS.map((key) => (
                            <Box
                                component="dl"
                                key={key}
                                className={`audit-compare-faq-item audit-compare-faq-${key}`}
                                sx={{m: 0}}
                            >
                                <Box
                                    component="dt"
                                    className="audit-compare-faq-question"
                                    sx={{fontSize: 14, lineHeight: "20px", fontWeight: 600}}
                                >
                                    {t(`${KEYS}.faq.${key}.question`)}
                                </Box>
                                <Box
                                    component="dd"
                                    className="audit-compare-faq-answer"
                                    sx={{
                                        m: "2px 0 0",
                                        fontSize: 14,
                                        lineHeight: "21px",
                                        color: PALETTE.muted,
                                    }}
                                >
                                    {t(`${KEYS}.faq.${key}.answer`)}
                                </Box>
                            </Box>
                        ))}
                        <Box
                            className="audit-compare-tip"
                            sx={{
                                gridColumn: "1 / -1",
                                display: "flex",
                                alignItems: "flex-start",
                                gap: "10px",
                                p: "10px 12px",
                                borderRadius: "6px",
                                backgroundColor: PALETTE.tint,
                                fontSize: 13,
                                lineHeight: "19px",
                                color: "#334155",
                            }}
                        >
                            <svg
                                className="audit-compare-tip-icon"
                                width="16"
                                height="16"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke={PALETTE.navy}
                                strokeWidth="2"
                                strokeLinecap="round"
                                strokeLinejoin="round"
                                aria-hidden="true"
                                style={{flex: "none", marginTop: 1}}
                            >
                                <rect
                                    className="audit-compare-tip-icon-phone"
                                    x="7"
                                    y="2.5"
                                    width="10"
                                    height="19"
                                    rx="2"
                                />
                                <path className="audit-compare-tip-icon-line" d="M11 18.5h2" />
                            </svg>
                            <span className="audit-compare-tip-text">{t(`${KEYS}.faq.tip`)}</span>
                        </Box>
                    </Box>
                ) : null}
            </Box>
        </Box>
    )
}
