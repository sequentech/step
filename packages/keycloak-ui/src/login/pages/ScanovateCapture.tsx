// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, useReducer, useRef, useState, type Dispatch, type ReactNode} from "react"
import {kcSanitize} from "keycloakify/lib/kcSanitize"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import {SymbolTone, TemplateLayout, type TemplateLabel} from "../Template"
import {ArrowIcon, CameraOffIcon, CheckIcon, SwitchCameraIcon} from "../icons"
import type {MessageKey} from "../i18n"
import {
    acceptsMessage,
    captureEvent,
    captureReducer,
    initialCaptureState,
    parseScanovateMessage,
    type CaptureEvent,
} from "../scanovate/capture"
import {
    CameraFacing,
    cameraCount,
    canChooseCamera,
    otherFacing,
    startingFacing,
    storeFacing,
    storedFacing,
    watchFrame,
} from "../scanovate/camera"
import {ACTION_PARAM, CaptureAction} from "../scanovate/form"
import type {ScanovatePageProps} from "../scanovate/pageProps"
import {HoldingCamera} from "../scanovate/HoldingCamera"
import {AttemptsLeft} from "../scanovate/parts"
import {EnrollmentStep, documentName, enrollmentFrame, textFor, type Text} from "../scanovate/text"
import {CaptureProblem, CaptureStage} from "../scanovate/types"
import {stepBack} from "../stepBack"

const PROBLEMS: Record<CaptureProblem, [MessageKey, MessageKey]> = {
    [CaptureProblem.Camera]: ["scanovateCameraProblemTitle", "scanovateCameraProblemText"],
    [CaptureProblem.Expired]: ["scanovateSessionExpiredTitle", "scanovateSessionExpiredText"],
    [CaptureProblem.InvalidToken]: ["scanovateStartAgainTitle", "scanovateStartAgainText"],
    [CaptureProblem.Closed]: ["scanovateClosedTitle", "scanovateClosedText"],
    [CaptureProblem.Upload]: ["scanovateUploadProblemTitle", "scanovateUploadProblemText"],
    [CaptureProblem.Generic]: ["scanovateProblemTitle", "scanovateProblemText"],
}

// The embedded Scanovate page of each step, and how the page presents it.
type FrameStage = CaptureStage.Document | CaptureStage.Face
const FRAMES: Record<FrameStage, {title: MessageKey; lead: MessageKey; frame: MessageKey}> = {
    [CaptureStage.Document]: {
        title: "scanovateDocumentStepTitle",
        lead: "scanovateDocumentStepLead",
        frame: "scanovateDocumentFrameTitle",
    },
    [CaptureStage.Face]: {
        title: "scanovateFaceStepTitle",
        lead: "scanovateFaceStepLead",
        frame: "scanovateFaceFrameTitle",
    },
}

export default function ScanovateCapture(props: ScanovatePageProps<"scanovate-capture.ftl">) {
    const {kcContext, i18n, Template, doUseDefaultCss, classes} = props
    const {scanovate, url} = kcContext
    const text = textFor(kcContext, i18n)
    const [state, dispatch] = useReducer(
        captureReducer,
        scanovate.holding !== undefined,
        initialCaptureState
    )
    const formRef = useRef<HTMLFormElement | null>(null)
    // The camera of the document scan: as last chosen on this device, and
    // until then the one the realm starts with.
    const [facing, setFacing] = useState(() =>
        storedFacing(deviceStorage(), startingFacing(scanovate.documentCamera))
    )
    const cameras = useCameraCount(state.stage === CaptureStage.Document)

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

    // The three camera steps share one layout: the camera over the whole
    // window, so it is what the voter sees from the start of each step.
    const page = {kcContext, i18n, doUseDefaultCss, classes, Template}

    if (state.stage === CaptureStage.Holding && scanovate.holding !== undefined) {
        return (
            <CaptureWindow
                {...page}
                title={text("scanovateHoldingStepTitle")}
                lead={text(
                    "scanovateHoldingStepLead",
                    documentName(kcContext, i18n, scanovate.documentType).text
                )}
                headingShown
            >
                <HoldingCamera
                    key={state.attempt}
                    text={text}
                    holding={scanovate.holding}
                    dispatch={dispatch}
                />
            </CaptureWindow>
        )
    }

    const frame = FRAMES[state.stage as FrameStage]
    const service = state.stage === CaptureStage.Document ? scanovate.ocr : scanovate.liveness
    const message = state.stage === CaptureStage.Document ? kcContext.message : undefined
    const attemptsUsed = scanovate.attemptsLeft < scanovate.maxAttempts
    // Only the document scan picks a side, and only a page of this origin can
    // be told which. The face check opens the front camera by itself.
    const cameraChoice =
        state.stage === CaptureStage.Document && canChooseCamera(service.url, window.location.href)
    const switchLabel = text(
        facing === CameraFacing.Rear ? "scanovateUseFrontCamera" : "scanovateUseRearCamera"
    )
    return (
        <CaptureWindow
            {...page}
            title={text(frame.title)}
            lead={text(frame.lead, documentName(kcContext, i18n, scanovate.documentType).text)}
        >
            <ScanovateFrame
                // A new mount opens a new Scanovate session with the same token.
                // So does the other camera: the page opens its camera once.
                key={`${state.stage}-${state.attempt}-${cameraChoice ? facing : ""}`}
                url={service.url}
                title={text(frame.frame)}
                dispatch={dispatch}
                facing={cameraChoice ? facing : undefined}
            />
            {cameraChoice && cameras > 1 && (
                <button
                    type="button"
                    className="scanovate-camera-switch"
                    lang={switchLabel.lang}
                    onClick={() => {
                        const chosen = otherFacing(facing)
                        storeFacing(deviceStorage(), chosen)
                        setFacing(chosen)
                    }}
                >
                    <SwitchCameraIcon />
                    <span>{switchLabel.text}</span>
                </button>
            )}
            {(message !== undefined || attemptsUsed) && (
                <div className="scanovate-window-note">
                    {message !== undefined && (
                        <p
                            role={message.type === "error" ? "alert" : "status"}
                            dangerouslySetInnerHTML={{__html: kcSanitize(message.summary)}}
                        />
                    )}
                    {attemptsUsed && (
                        <AttemptsLeft text={text} attemptsLeft={scanovate.attemptsLeft} />
                    )}
                </div>
            )}
        </CaptureWindow>
    )
}

/** The browser's storage for this site, where there is one. */
function deviceStorage(): Storage | undefined {
    try {
        return window.localStorage
    } catch {
        return undefined
    }
}

/**
 * How many cameras the device has, while `watching`.
 *
 * A browser lists one camera until the voter has allowed the camera, which
 * happens inside Scanovate's page, so the count is taken again every second
 * until there is more than one.
 */
function useCameraCount(watching: boolean): number {
    const [count, setCount] = useState(0)
    useEffect(() => {
        if (!watching || count > 1) return
        let stopped = false
        const look = () =>
            void cameraCount(navigator.mediaDevices).then((found) => {
                if (!stopped) setCount(found)
            })
        look()
        const timer = window.setInterval(look, 1000)
        return () => {
            stopped = true
            window.clearInterval(timer)
        }
    }, [watching, count])
    return count
}

/**
 * The layout of every camera step: the camera covers the whole window.
 *
 * The face check needs it (ID R&D's face capture inside Scanovate's page takes
 * a frame much smaller than the window for open DevTools, and refuses to
 * start), and the other steps follow it so that the voter sees one layout. The
 * way out of a step is its close button, which leads to the card that offers
 * to try again, start over or go back.
 */
function CaptureWindow(
    props: Pick<
        ScanovatePageProps<"scanovate-capture.ftl">,
        "kcContext" | "i18n" | "doUseDefaultCss" | "classes" | "Template"
    > & {
        title: TemplateLabel
        lead: TemplateLabel
        // Scanovate's pages show their own title: only the page's own camera shows this one.
        headingShown?: boolean
        children: ReactNode
    }
) {
    const {Template, title, lead, headingShown = false, children, ...template} = props
    return (
        <Template
            {...template}
            layout={TemplateLayout.Fullscreen}
            displayMessage={false}
            headerNode={title.text}
        >
            <div className="scanovate-window">
                {children}
                <div className={headingShown ? "scanovate-window-heading" : "auth-visually-hidden"}>
                    <h1 id="kc-page-title" lang={title.lang}>
                        {title.text}
                    </h1>
                    <p lang={lead.lang}>{lead.text}</p>
                </div>
            </div>
        </Template>
    )
}

/**
 * One of Scanovate's own capture pages, embedded as its integration guide asks,
 * over the whole capture window.
 */
function ScanovateFrame(props: {
    url: string
    title: TemplateLabel
    dispatch: Dispatch<CaptureEvent>
    // The camera the page is to open, where this page can tell it.
    facing?: CameraFacing
}) {
    const {url, title, dispatch, facing} = props
    const ref = useRef<HTMLIFrameElement | null>(null)

    // The rear camera is what the page opens by itself: nothing to tell it.
    useEffect(() => {
        if (facing !== CameraFacing.Front || ref.current === null) return
        return watchFrame(ref.current, facing)
    }, [facing])

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
