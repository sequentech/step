// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useState} from "react"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import {SymbolTone} from "../Template"
import {ArrowIcon, CopyIcon, WarningIcon} from "../icons"
import type {MessageKey} from "../i18n"
import type {ScanovatePageProps} from "../scanovate/pageProps"
import {EnrollmentStep, dynamicText, enrollmentFrame, textFor, type Text} from "../scanovate/text"
import {AttemptsLeft, CheckList} from "../scanovate/parts"

const RETRY_ACTION = "retry"

const CAPTURE_TIPS: MessageKey[] = [
    "scanovateTipLight",
    "scanovateTipCorners",
    "scanovateTipSteady",
]

// Tips for the reason the last attempt failed, by the error's message key.
const TIPS: Record<string, MessageKey[]> = {
    scanovateDocumentAuthenticationError: CAPTURE_TIPS,
    scanovateScoringError: CAPTURE_TIPS,
    scanovateCaptureInvalidError: CAPTURE_TIPS,
    scanovateVerificationFailedError: [
        "scanovateTipValidDocument",
        "scanovateTipFaceUncovered",
        "scanovateTipLookAtCamera",
    ],
    scanovateAttributesError: ["scanovateTipSameDocument", "scanovateTipCheckDetails"],
    scanovateMaxTrialsError: ["scanovateTipCamera", "scanovateTipConnection", "scanovateTipLight"],
    scanovateInternalError: ["scanovateTipWait", "scanovateTipConnection"],
}

export function tipsFor(error: string): MessageKey[] {
    return TIPS[error] ?? CAPTURE_TIPS
}

enum CopyState {
    Idle = "IDLE",
    Copied = "COPIED",
    Failed = "FAILED",
}

function SupportReference({text, code}: {text: Text; code: string}) {
    const [copy, setCopy] = useState(CopyState.Idle)
    const label = text("scanovateSupportReferenceLabel")
    const button = text("scanovateCopy")
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
            ? text("scanovateCopied")
            : copy === CopyState.Failed
              ? text("scanovateCopyFailed")
              : null
    return (
        <div className="auth-reference">
            <span id="scanovate-reference-label" lang={label.lang}>
                {label.text}
            </span>
            <code aria-describedby="scanovate-reference-label">{code}</code>
            <div className="auth-reference-actions">
                <button
                    type="button"
                    onClick={() => void copyCode()}
                    aria-label={text("scanovateCopyReference").text}
                    lang={button.lang}
                >
                    <CopyIcon />
                    {button.text}
                </button>
                <span role="status" lang={feedback?.lang}>
                    {feedback?.text}
                </span>
            </div>
        </div>
    )
}

export default function ScanovateError(props: ScanovatePageProps<"scanovate-error.ftl">) {
    const {kcContext, i18n, Template, doUseDefaultCss, classes} = props
    const {url, error, canRetry, code_id: code, attemptsLeft} = kcContext
    const text = textFor(kcContext, i18n)
    const title = text(canRetry ? "scanovateErrorHeading" : "scanovateErrorFinalHeading")
    const reason = dynamicText(kcContext, i18n, error)
    const tipsTitle = text("scanovateErrorTipsTitle")
    const retry = text("scanovateTryAgain")

    return (
        <Template
            kcContext={kcContext}
            i18n={i18n}
            doUseDefaultCss={doUseDefaultCss}
            classes={classes}
            {...enrollmentFrame(kcContext, i18n, EnrollmentStep.VerifyIdentity)}
            symbol={<WarningIcon />}
            symbolTone={SymbolTone.Warning}
            headerNode={title.text}
            titleLang={title.lang}
        >
            <p className="auth-lead" lang={reason.lang}>
                {reason.text}
            </p>
            {canRetry && (
                <>
                    <section className="auth-panel" aria-labelledby="scanovate-tips">
                        <h2 id="scanovate-tips" className="auth-panel-title" lang={tipsTitle.lang}>
                            {tipsTitle.text}
                        </h2>
                        <CheckList items={tipsFor(error).map((key) => text(key))} />
                    </section>
                    <Box
                        component="form"
                        method="post"
                        action={url.loginAction}
                        className="auth-actions"
                    >
                        <input type="hidden" name="action" value={RETRY_ACTION} />
                        <Button
                            type="submit"
                            variant="contained"
                            fullWidth
                            className="auth-submit"
                            endIcon={<ArrowIcon />}
                            lang={retry.lang}
                        >
                            {retry.text}
                        </Button>
                        {attemptsLeft !== undefined && attemptsLeft > 0 && (
                            <AttemptsLeft text={text} attemptsLeft={attemptsLeft} />
                        )}
                    </Box>
                </>
            )}
            {code !== undefined && code !== "" && <SupportReference text={text} code={code} />}
        </Template>
    )
}
