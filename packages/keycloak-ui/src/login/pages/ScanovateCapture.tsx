// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, useReducer, useRef, type Dispatch, type ReactNode} from "react"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import {SymbolTone, type TemplateLabel} from "../Template"
import {ArrowIcon, CameraOffIcon, CheckIcon, FaceScanIcon, IdCardIcon} from "../icons"
import type {MessageKey} from "../i18n"
import {
    acceptsMessage,
    captureEvent,
    captureReducer,
    initialCaptureState,
    parseScanovateMessage,
    type CaptureEvent,
} from "../scanovate/capture"
import {ACTION_PARAM, CaptureAction} from "../scanovate/form"
import type {ScanovatePageProps} from "../scanovate/pageProps"
import {AttemptsLeft} from "../scanovate/parts"
import {EnrollmentStep, documentName, enrollmentFrame, textFor, type Text} from "../scanovate/text"
import {CaptureProblem, CaptureStage} from "../scanovate/types"
import {stepBack} from "../stepBack"

const PROBLEMS: Record<CaptureProblem, [MessageKey, MessageKey]> = {
    [CaptureProblem.Camera]: ["scanovateCameraProblemTitle", "scanovateCameraProblemText"],
    [CaptureProblem.Expired]: ["scanovateSessionExpiredTitle", "scanovateSessionExpiredText"],
    [CaptureProblem.InvalidToken]: ["scanovateStartAgainTitle", "scanovateStartAgainText"],
    [CaptureProblem.Closed]: ["scanovateClosedTitle", "scanovateClosedText"],
    [CaptureProblem.Generic]: ["scanovateProblemTitle", "scanovateProblemText"],
}

// The embedded Scanovate page of each step, and how the page presents it.
type FrameStage = CaptureStage.Document | CaptureStage.Face
const FRAMES: Record<
    FrameStage,
    {title: MessageKey; lead: MessageKey; frame: MessageKey; symbol: () => ReactNode}
> = {
    [CaptureStage.Document]: {
        title: "scanovateDocumentStepTitle",
        lead: "scanovateDocumentStepLead",
        frame: "scanovateDocumentFrameTitle",
        symbol: IdCardIcon,
    },
    [CaptureStage.Face]: {
        title: "scanovateFaceStepTitle",
        lead: "scanovateFaceStepLead",
        frame: "scanovateFaceFrameTitle",
        symbol: FaceScanIcon,
    },
}

export default function ScanovateCapture(props: ScanovatePageProps<"scanovate-capture.ftl">) {
    const {kcContext, i18n, Template, doUseDefaultCss, classes} = props
    const {scanovate, url} = kcContext
    const text = textFor(kcContext, i18n)
    const [state, dispatch] = useReducer(captureReducer, undefined, initialCaptureState)
    const formRef = useRef<HTMLFormElement | null>(null)

    // Keycloak has the results from Scanovate already: the form only moves the flow on.
    useEffect(() => {
        if (state.stage !== CaptureStage.Checking) return
        const form = formRef.current
        if (form === null) return
        // Safari before 16 has no requestSubmit.
        if ("requestSubmit" in form) {
            form.requestSubmit()
        } else {
            HTMLFormElement.prototype.submit.call(form)
        }
    }, [state.stage])

    const card = {
        kcContext,
        i18n,
        doUseDefaultCss,
        classes,
        ...enrollmentFrame(kcContext, i18n, EnrollmentStep.VerifyIdentity),
        back: stepBack(kcContext, text("enrollmentBack")),
    }

    if (state.stage === CaptureStage.Checking) {
        const title = text("scanovateCheckingTitle")
        return (
            <Template
                {...card}
                displayMessage={false}
                symbol={<span className="spin" />}
                headerNode={title.text}
                titleLang={title.lang}
            >
                <Checking text={text} />
                <form ref={formRef} method="post" action={url.loginAction} hidden>
                    <input type="hidden" name={ACTION_PARAM} value={CaptureAction.Capture} />
                </form>
            </Template>
        )
    }

    if (state.problem !== null) {
        const [titleKey, bodyKey] = PROBLEMS[state.problem]
        const title = text(titleKey)
        const body = text(bodyKey)
        return (
            <Template
                {...card}
                displayMessage={false}
                symbol={<CameraOffIcon />}
                symbolTone={SymbolTone.Warning}
                headerNode={title.text}
                titleLang={title.lang}
            >
                <p className="auth-lead" lang={body.lang}>
                    {body.text}
                </p>
                <ProblemActions
                    text={text}
                    loginAction={url.loginAction}
                    canTryAgain={state.problem !== CaptureProblem.InvalidToken}
                    onTryAgain={() => dispatch({type: "tryAgain"})}
                />
            </Template>
        )
    }

    const frame = FRAMES[state.stage]
    const title = text(frame.title)
    const lead = text(frame.lead, documentName(kcContext, i18n, scanovate.documentType).text)
    const Symbol = frame.symbol
    const service = state.stage === CaptureStage.Document ? scanovate.ocr : scanovate.liveness
    return (
        <Template
            {...card}
            displayMessage={state.stage === CaptureStage.Document}
            symbol={<Symbol />}
            headerNode={title.text}
            titleLang={title.lang}
        >
            <p className="auth-lead" lang={lead.lang}>
                {lead.text}
            </p>
            {scanovate.attemptsLeft < scanovate.maxAttempts && (
                <AttemptsLeft text={text} attemptsLeft={scanovate.attemptsLeft} />
            )}
            <ScanovateFrame
                // A new mount opens a new Scanovate session with the same token.
                key={`${state.stage}-${state.attempt}`}
                url={service.url}
                title={text(frame.frame)}
                dispatch={dispatch}
            />
            <StartOver text={text} loginAction={url.loginAction} />
        </Template>
    )
}

/** One of Scanovate's own capture pages, embedded as its integration guide asks. */
function ScanovateFrame(props: {
    url: string
    title: TemplateLabel
    dispatch: Dispatch<CaptureEvent>
}) {
    const {url, title, dispatch} = props
    const ref = useRef<HTMLIFrameElement | null>(null)

    useEffect(() => {
        const listen = (event: MessageEvent) => {
            if (!acceptsMessage(event, ref.current?.contentWindow, window.location.origin, url)) {
                return
            }
            const message = parseScanovateMessage(event.data)
            if (message !== null) dispatch(captureEvent(message))
        }
        window.addEventListener("message", listen)
        return () => window.removeEventListener("message", listen)
    }, [url, dispatch])

    return (
        <div className="scanovate-frame">
            <iframe
                ref={ref}
                src={url}
                title={title.text}
                lang={title.lang}
                allow="camera *; microphone *"
                onLoad={(event) => event.currentTarget.contentWindow?.focus()}
            />
        </div>
    )
}

function ProblemActions(props: {
    text: Text
    loginAction: string
    canTryAgain: boolean
    onTryAgain: () => void
}) {
    const {text, loginAction, canTryAgain, onTryAgain} = props
    const tryAgain = text("scanovateTryAgain")
    const startOver = text("scanovateStartOver")
    // Start over: new one-time tokens from Keycloak, without using up an attempt.
    return (
        <Box component="form" method="post" action={loginAction} className="auth-actions">
            <input type="hidden" name={ACTION_PARAM} value={CaptureAction.Retry} />
            {canTryAgain && (
                <Button
                    type="button"
                    variant="contained"
                    fullWidth
                    className="auth-submit"
                    endIcon={<ArrowIcon />}
                    onClick={onTryAgain}
                    lang={tryAgain.lang}
                >
                    {tryAgain.text}
                </Button>
            )}
            <Button
                type="submit"
                variant={canTryAgain ? "outlined" : "contained"}
                fullWidth
                className={canTryAgain ? undefined : "auth-submit"}
                endIcon={canTryAgain ? undefined : <ArrowIcon />}
                lang={startOver.lang}
            >
                {startOver.text}
            </Button>
        </Box>
    )
}

// For a voter stuck in a Scanovate page: new one-time tokens, without using up an attempt.
function StartOver({text, loginAction}: {text: Text; loginAction: string}) {
    const label = text("scanovateStartOver")
    return (
        <form method="post" action={loginAction} className="scanovate-start-over">
            <input type="hidden" name={ACTION_PARAM} value={CaptureAction.Retry} />
            <Button type="submit" variant="text" lang={label.lang}>
                {label.text}
            </Button>
        </form>
    )
}

function Checking({text}: {text: Text}) {
    const lead = text("scanovateCheckingLead")
    const steps: [MessageKey, string][] = [
        ["scanovateCheckingScanned", "done"],
        ["scanovateCheckingVerifying", "current"],
        ["scanovateCheckingReading", ""],
    ]
    return (
        <>
            <p className="auth-lead" lang={lead.lang} role="status">
                {lead.text}
            </p>
            <ol className="auth-timeline">
                {steps.map(([key, state]) => {
                    const label = text(key)
                    return (
                        <li
                            key={key}
                            className={state || undefined}
                            aria-current={state === "current" ? "step" : undefined}
                        >
                            <span className="mark" aria-hidden="true">
                                {state === "done" && <CheckIcon />}
                            </span>
                            <div>
                                <strong lang={label.lang}>{label.text}</strong>
                            </div>
                        </li>
                    )
                })}
            </ol>
        </>
    )
}
