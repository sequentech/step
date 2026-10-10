// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useState, type ReactNode} from "react"
import {SymbolTone} from "../Template"
import {CheckCircleIcon, ClockIcon, CopyIcon, WarningIcon} from "../icons"
import type {MessageKey} from "../i18n"
import {inChannelOrder, type EnrollmentOutcome} from "../KcContext"
import type {EnrollmentPageProps} from "../enrollment/pageProps"
import {dynamicText, finishedFrame, textFor, type Text} from "../scanovate/text"

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

// The application's support reference: 8 capital letters and digits, shown as "XXXX-XXXX".
const SUPPORT_REFERENCE = /^([A-Z0-9]{4})-?([A-Z0-9]{4})$/

/** The reference as the voter reads it out, or none for any other value. */
export function supportReferenceOf(value: string | undefined): string | undefined {
    const match = value === undefined ? null : SUPPORT_REFERENCE.exec(value)
    return match === null ? undefined : `${match[1]}-${match[2]}`
}

enum CopyState {
    Idle = "IDLE",
    Copied = "COPIED",
    Failed = "FAILED",
}

const canCopy = () =>
    typeof navigator !== "undefined" && typeof navigator.clipboard?.writeText === "function"

// Stands for the reference in the translated sentence, so it can be shown as code.
const REFERENCE_SLOT = "\u0000"

/** "Need help? … give them this reference code: Q5KW-8RFX", with a button to copy it. */
function SupportReference({text, code}: {text: Text; code: string}) {
    const [copy, setCopy] = useState(CopyState.Idle)
    const sentence = text("messageOtp.pending.reference", REFERENCE_SLOT)
    const [before, after = ""] = sentence.text.split(REFERENCE_SLOT)
    const button = text("messageOtp.pending.copyReference")
    const copyCode = async () => {
        try {
            await navigator.clipboard.writeText(code)
            setCopy(CopyState.Copied)
        } catch {
            setCopy(CopyState.Failed)
        }
    }
    const feedback =
        copy === CopyState.Copied
            ? text("messageOtp.pending.referenceCopied")
            : copy === CopyState.Failed
              ? text("scanovateCopyFailed")
              : null
    return (
        <div className="auth-reference">
            <span id="support-reference" lang={sentence.lang}>
                {before}
                <code>{code}</code>
                {after}
            </span>
            {canCopy() && (
                <div className="auth-reference-actions">
                    <button type="button" onClick={() => void copyCode()} lang={button.lang}>
                        <CopyIcon />
                        {button.text}
                    </button>
                    <span role="status" lang={feedback?.lang}>
                        {feedback?.text}
                    </span>
                </div>
            )}
        </div>
    )
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
    // Where the result goes: the channel the voter verified, with its masked contact.
    const [noticeChannel] = inChannelOrder(
        kcContext.noticeChannel === undefined ? [] : [kcContext.noticeChannel]
    )
    const noticeLabel =
        noticeChannel === undefined ? undefined : i18n.msgStr(`messageChannel.${noticeChannel}`)
    const notice =
        noticeLabel === undefined
            ? undefined
            : kcContext.noticeAddress
              ? text("messageOtp.pending.channelAddress", noticeLabel, kcContext.noticeAddress)
              : text("messageOtp.pending.channel", noticeLabel)
    const supportReference = supportReferenceOf(kcContext.supportReference)
    // The review page's own text only says "SMS or email": the channel line replaces it.
    const showMessage = notice === undefined || pageId !== "registration-manual-finish.ftl"

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
            {showMessage && (
                <p className="auth-message" lang={message.lang}>
                    {i18n.msg(finish.message)}
                </p>
            )}
            {notice !== undefined && (
                <p id="notice-channel" className="auth-message" lang={notice.lang}>
                    {notice.text}
                </p>
            )}
            {supportReference !== undefined && (
                <SupportReference text={text} code={supportReference} />
            )}
        </Template>
    )
}
