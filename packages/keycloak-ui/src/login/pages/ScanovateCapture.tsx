// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {
    useEffect,
    useReducer,
    useRef,
    useState,
    type Dispatch,
    type ReactNode,
    type RefObject,
} from "react"
import {kcSanitize} from "keycloakify/lib/kcSanitize"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import {SymbolTone, TemplateLayout, type TemplateLabel} from "../Template"
import {ArrowIcon, CameraOffIcon, CheckIcon} from "../icons"
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
    canChooseCamera,
    frameDevices,
    mostCameras,
    otherFacing,
    startingFacing,
    deviceStorage,
    storeFacing,
    storedFacing,
    watchFrame,
} from "../scanovate/camera"
import {ACTION_PARAM, CaptureAction} from "../scanovate/form"
import type {ScanovatePageProps} from "../scanovate/pageProps"
import {HoldingCamera} from "../scanovate/HoldingCamera"
import {keepLaidOut} from "../scanovate/layout"
import {AttemptsLeft, CameraSwitch} from "../scanovate/parts"
import {EnrollmentStep, documentName, enrollmentFrame, textFor, type Text} from "../scanovate/text"
import {CaptureProblem, CaptureStage, ScanovateMessageKind} from "../scanovate/types"
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
    const frameRef = useRef<HTMLIFrameElement | null>(null)
    const cameras = useCameraCount(state.stage === CaptureStage.Document, frameRef)

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
    const ownOrigin = (url: string) => canChooseCamera(url, window.location.href)
    // Only the document scan picks a side, and only a page of this origin can
    // be told which. The face check opens the front camera by itself, and its
    // page is left as it is: it looks for changes to how the camera is opened.
    const cameraChoice = state.stage === CaptureStage.Document && ownOrigin(service.url)
    // Pages of this origin follow each other in one frame: Safari remembers
    // that the voter allowed the camera for the frame that asked, and a new
    // frame for the face check has it ask the voter again.
    const oneFrame = ownOrigin(scanovate.ocr.url) && ownOrigin(scanovate.liveness.url)
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
                key={
                    oneFrame
                        ? `${state.attempt}-${facing}`
                        : `${state.stage}-${state.attempt}-${cameraChoice ? facing : ""}`
                }
                frameRef={frameRef}
                url={service.url}
                title={text(frame.frame)}
                dispatch={dispatch}
                facing={cameraChoice ? facing : undefined}
                // The document page's layout does not always follow a turn of
                // the device by itself. Only a page of this origin can be asked.
                laidOutAgain={cameraChoice}
            />
            {cameraChoice && cameras > 1 && (
                <CameraSwitch
                    label={switchLabel}
                    onClick={() => {
                        const chosen = otherFacing(facing)
                        storeFacing(deviceStorage(), chosen)
                        setFacing(chosen)
                    }}
                />
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

/**
 * How many cameras the device has, while `watching`.
 *
 * A browser lists one camera until the voter has allowed the camera, which
 * happens inside Scanovate's page, so the count is taken again every second
 * until there is more than one; and Safari goes on listing one to this page,
 * so the count is also taken where the camera was allowed, in the frame. Once
 * there is more than one the count stays: the switch is not taken away.
 */
function useCameraCount(watching: boolean, frame: RefObject<HTMLIFrameElement | null>): number {
    const [count, setCount] = useState(0)
    useEffect(() => {
        if (!watching || count > 1) return
        let stopped = false
        const look = () =>
            void mostCameras([navigator.mediaDevices, frameDevices(frame.current)]).then(
                (found) => {
                    if (!stopped) setCount(found)
                }
            )
        look()
        const timer = window.setInterval(look, 1000)
        return () => {
            stopped = true
            window.clearInterval(timer)
        }
    }, [watching, count, frame])
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

/** The document a frame shows; null for a page of another origin. */
function frameDocument(frame: HTMLIFrameElement | null): Document | null {
    try {
        return frame?.contentDocument ?? null
    } catch {
        return null
    }
}

/**
 * One of Scanovate's own capture pages, embedded as its integration guide asks,
 * over the whole capture window.
 *
 * While it stays mounted, a new `url` opens in the same frame, in place of
 * the page that finished: no step is added to the browser's Back.
 */
function ScanovateFrame(props: {
    frameRef: RefObject<HTMLIFrameElement | null>
    url: string
    title: TemplateLabel
    dispatch: Dispatch<CaptureEvent>
    // The camera the page is to open, where this page can tell it.
    facing?: CameraFacing
    // Whether to have the page lay itself out again after the device is turned.
    laidOutAgain?: boolean
}) {
    const {frameRef: ref, url, title, dispatch, facing, laidOutAgain = false} = props
    // The page the frame starts with; later ones are opened in it.
    const [first] = useState(url)
    const shown = useRef(url)
    // The page that finished has no more to say, while the next one loads.
    const finished = useRef<Document | null>(null)

    useEffect(() => {
        if (shown.current === url) return
        shown.current = url
        finished.current = frameDocument(ref.current)
        ref.current?.contentWindow?.location.replace(new URL(url, window.location.href).href)
    }, [url, ref])

    // The rear camera is what the page opens by itself: nothing to tell it.
    useEffect(() => {
        if (facing !== CameraFacing.Front || ref.current === null) return
        return watchFrame(ref.current, facing)
    }, [facing, ref])

    useEffect(() => {
        if (!laidOutAgain || ref.current === null) return
        return keepLaidOut(ref.current)
    }, [laidOutAgain, ref])

    useEffect(() => {
        const listen = (event: MessageEvent) => {
            if (!acceptsMessage(event, ref.current?.contentWindow, window.location.origin, url)) {
                return
            }
            const message = parseScanovateMessage(event.data)
            if (message === null) return
            const speaking = frameDocument(ref.current)
            if (speaking !== null && speaking === finished.current) return
            if (message.kind === ScanovateMessageKind.Done) finished.current = speaking
            dispatch(captureEvent(message))
        }
        window.addEventListener("message", listen)
        return () => window.removeEventListener("message", listen)
    }, [url, dispatch, ref])

    return (
        <div className="scanovate-frame">
            <iframe
                ref={ref}
                src={first}
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
