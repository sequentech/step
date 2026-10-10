// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {ReactNode} from "react"
import {SymbolTone} from "../Template"
import {CheckCircleIcon, ClockIcon, WarningIcon} from "../icons"
import type {MessageKey} from "../i18n"
import type {EnrollmentOutcome} from "../KcContext"
import type {EnrollmentPageProps} from "../enrollment/pageProps"
import {dynamicText, finishedFrame, textFor} from "../scanovate/text"

type FinishPageId =
    | "registration-finish.ftl"
    | "registration-manual-finish.ftl"
    | "registration-rejected-finish.ftl"
    | "message-finish.ftl"

type Finish = {
    outcome: MessageKey
    title: MessageKey
    message: MessageKey
    symbol: ReactNode
    tone: SymbolTone
}

const FINISHES: Record<FinishPageId, Finish> = {
    "registration-finish.ftl": {
        outcome: "enrollmentOutcomeEnrolled",
        title: "registerFinishTitle",
        message: "registerFinishMessage",
        symbol: <CheckCircleIcon />,
        tone: SymbolTone.Default,
    },
    "registration-manual-finish.ftl": {
        outcome: "enrollmentOutcomeUnderReview",
        title: "registerFinishManualTitle",
        message: "registerFinishManualMessage",
        symbol: <ClockIcon />,
        tone: SymbolTone.Default,
    },
    "registration-rejected-finish.ftl": {
        outcome: "enrollmentOutcomeRejected",
        title: "registerFinishRejectedTitle",
        message: "registerFinishRejectedMessage",
        symbol: <WarningIcon />,
        tone: SymbolTone.Warning,
    },
    "message-finish.ftl": {
        outcome: "enrollmentOutcomeValidated",
        title: "messageFinishTitle",
        message: "messageFinish",
        symbol: <CheckCircleIcon />,
        tone: SymbolTone.Default,
    },
}

const NO_VOTER = "NO_VOTER"

// The fields to list. Without a voter in the registry no field was compared,
// so a review has none to show.
export function mismatchesOf(
    pageId: FinishPageId,
    outcome: EnrollmentOutcome | undefined
): EnrollmentOutcome["mismatchedFields"] {
    if (outcome === undefined) return []
    if (pageId === "registration-manual-finish.ftl" && outcome.reason === NO_VOTER) return []
    return outcome.mismatchedFields
}

// How an enrollment ended: approved, waiting for a review or rejected.
export default function EnrollmentFinish(props: EnrollmentPageProps<FinishPageId>) {
    const {kcContext, i18n, Template, doUseDefaultCss, classes} = props
    const {url, pageId, enrollmentOutcome} = kcContext
    const finish = FINISHES[pageId]
    const text = textFor(kcContext, i18n)
    const title = text(finish.title)
    const message = text(finish.message)
    const reason = enrollmentOutcome?.reason
    const mismatches = mismatchesOf(pageId, enrollmentOutcome)
    const mismatchesTitle = text("rejectReasonListItems")
    const empty = text("empty")

    return (
        <Template
            kcContext={kcContext}
            i18n={i18n}
            doUseDefaultCss={doUseDefaultCss}
            classes={classes}
            {...finishedFrame(kcContext, i18n, finish.outcome)}
            symbol={finish.symbol}
            symbolTone={finish.tone}
            headerNode={title.text}
            titleLang={title.lang}
            displayMessage={false}
            displayInfo
            infoNode={
                <p id="instruction1">
                    {i18n.msgStr("pageExpiredMsg2")}{" "}
                    <a id="loginContinueLink" href={url.loginRestartFlowUrl}>
                        {i18n.msgStr("doClickHere")}
                    </a>
                    .
                </p>
            }
        >
            {reason !== undefined && reason !== "" && (
                <p className="auth-lead" lang={dynamicText(kcContext, i18n, reason).lang}>
                    {i18n.advancedMsg(reason)}
                </p>
            )}
            {mismatches.length > 0 && (
                <section className="auth-panel" aria-labelledby="enrollment-mismatches">
                    <h2
                        id="enrollment-mismatches"
                        className="auth-panel-title"
                        lang={mismatchesTitle.lang}
                    >
                        {mismatchesTitle.text}
                    </h2>
                    <dl className="auth-summary">
                        {mismatches.map(({name, value}) => (
                            <div key={name}>
                                <dt>{name}</dt>
                                {value === null ? (
                                    <dd className="empty" lang={empty.lang}>
                                        {empty.text}
                                    </dd>
                                ) : (
                                    <dd>{value}</dd>
                                )}
                            </div>
                        ))}
                    </dl>
                </section>
            )}
            <p className="auth-message" lang={message.lang}>
                {i18n.msg(finish.message)}
            </p>
        </Template>
    )
}
