// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, useMemo, useReducer, useRef, useState, type ReactNode} from "react"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import Dialog from "@mui/material/Dialog"
import DialogActions from "@mui/material/DialogActions"
import DialogContent from "@mui/material/DialogContent"
import DialogTitle from "@mui/material/DialogTitle"
import useMediaQuery from "@mui/material/useMediaQuery"
import {SymbolTone, TemplateLayout, type TemplateLabel} from "../Template"
import {
    ArrowIcon,
    CameraOffIcon,
    CheckIcon,
    FaceIcon,
    FaceScanIcon,
    IdCardIcon,
    LockIcon,
    VideoIcon,
} from "../icons"
import type {MessageKey} from "../i18n"
import {ScanovateSide} from "../KcContext"
import CaptureCamera, {STEP_TITLES} from "../scanovate/CaptureCamera"
import LivenessFrame from "../scanovate/LivenessFrame"
import {AttemptsLeft, CheckList} from "../scanovate/parts"
import {
    Phase,
    captureReducer,
    currentStep,
    facingFor,
    initialCaptureState,
    stateAtStep,
    stepsFor,
} from "../scanovate/controller"
import {useCaptureEnvironment} from "../scanovate/environment"
import {CAPTURE_ACTION, CAPTURE_PARTS, MEDIA_STEPS, populateCaptureForm} from "../scanovate/form"
import {StageLayout} from "../scanovate/geometry"
import {stopStream, cameraProblem} from "../scanovate/media"
import type {ScanovatePageProps} from "../scanovate/pageProps"
import {EnrollmentStep, documentName, enrollmentFrame, textFor, type Text} from "../scanovate/text"
import {CameraFacing, CaptureProblem, CaptureStep, type Analyzers} from "../scanovate/types"

const FLASH_MS = 700
const TICK_MS = 1000
const PHONE_QUERY = "(max-width: 720px)"

const PROBLEMS: Record<CaptureProblem, [MessageKey, MessageKey]> = {
    [CaptureProblem.CameraDenied]: ["scanovateCameraDeniedTitle", "scanovateCameraDeniedText"],
    [CaptureProblem.CameraNotFound]: [
        "scanovateCameraNotFoundTitle",
        "scanovateCameraNotFoundText",
    ],
    [CaptureProblem.CameraInUse]: ["scanovateCameraInUseTitle", "scanovateCameraInUseText"],
    [CaptureProblem.InsecureContext]: [
        "scanovateInsecureContextTitle",
        "scanovateInsecureContextText",
    ],
    [CaptureProblem.CameraFailed]: ["scanovateCameraFailedTitle", "scanovateCameraFailedText"],
    [CaptureProblem.AnalyzerFailed]: [
        "scanovateAnalyzerFailedTitle",
        "scanovateAnalyzerFailedText",
    ],
    [CaptureProblem.RecorderUnsupported]: [
        "scanovateRecorderUnsupportedTitle",
        "scanovateRecorderUnsupportedText",
    ],
    [CaptureProblem.LivenessFailed]: [
        "scanovateLivenessFailedTitle",
        "scanovateLivenessFailedText",
    ],
    [CaptureProblem.LivenessExpired]: [
        "scanovateLivenessExpiredTitle",
        "scanovateLivenessExpiredText",
    ],
}

const HELP: Record<CaptureStep, MessageKey[]> = {
    [CaptureStep.Front]: [
        "scanovateHelpDocumentSurface",
        "scanovateHelpDocumentCorners",
        "scanovateHelpDocumentGlare",
        "scanovateHelpDocumentLight",
        "scanovateHelpShutter",
    ],
    [CaptureStep.Back]: [
        "scanovateHelpDocumentSurface",
        "scanovateHelpDocumentCorners",
        "scanovateHelpDocumentGlare",
        "scanovateHelpDocumentLight",
        "scanovateHelpShutter",
    ],
    [CaptureStep.Face]: [
        "scanovateHelpFaceLevel",
        "scanovateHelpFaceCoverings",
        "scanovateHelpFaceLight",
        "scanovateHelpFaceAlone",
    ],
    [CaptureStep.Video]: [
        "scanovateHelpVideoDocument",
        "scanovateHelpVideoFace",
        "scanovateHelpVideoStill",
    ],
    [CaptureStep.Liveness]: [
        "scanovateHelpFaceLevel",
        "scanovateHelpFaceCoverings",
        "scanovateHelpFaceLight",
        "scanovateHelpFaceAlone",
    ],
}

export default function ScanovateCapture(props: ScanovatePageProps<"scanovate-capture.ftl">) {
    const {kcContext, i18n, Template, doUseDefaultCss, classes} = props
    const {scanovate, url} = kcContext
    const {services, startAt} = useCaptureEnvironment()
    const text = textFor(kcContext, i18n)
    const phone = useMediaQuery(PHONE_QUERY, {noSsr: true})
    const liveness = scanovate.liveness
    const steps = useMemo(
        () => stepsFor(scanovate.sides, liveness !== undefined),
        [scanovate.sides, liveness]
    )
    const [state, dispatch] = useReducer(captureReducer, steps, (initial) =>
        startAt === undefined
            ? initialCaptureState(initial)
            : stateAtStep(initial, startAt, new Blob([], {type: "image/jpeg"}))
    )
    const step = currentStep(state)
    const facing = facingFor(step)
    const [analyzers, setAnalyzers] = useState<Analyzers | null>(null)
    const [analyzerFailed, setAnalyzerFailed] = useState(false)
    const [analyzerAttempt, setAnalyzerAttempt] = useState(0)
    const [stream, setStream] = useState<MediaStream | null>(null)
    const formRef = useRef<HTMLFormElement | null>(null)
    const cameraPhase = state.phase === Phase.Starting || state.phase === Phase.Capturing
    const problem =
        state.problem ?? (analyzerFailed && cameraPhase ? CaptureProblem.AnalyzerFailed : null)
    const needsCamera = cameraPhase && problem === null
    const document = documentName(kcContext, i18n, scanovate.documentType)

    useEffect(() => {
        let active = true
        let loaded: Analyzers | null = null
        const load = async () => {
            try {
                const result = await services.loadAnalyzers()
                if (!active) {
                    result.document.free()
                    result.face.free()
                    return
                }
                loaded = result
                setAnalyzers(result)
            } catch {
                if (active) setAnalyzerFailed(true)
            }
        }
        void load()
        return () => {
            active = false
            loaded?.document.free()
            loaded?.face.free()
            setAnalyzers(null)
        }
    }, [services, analyzerAttempt])

    useEffect(() => {
        if (!needsCamera) return
        let active = true
        let opened: MediaStream | null = null
        const open = async () => {
            try {
                const media = await services.camera.open(facing)
                if (!active) {
                    stopStream(media)
                    return
                }
                opened = media
                setStream(media)
                dispatch({type: "cameraReady", at: performance.now()})
            } catch (error) {
                if (active) dispatch({type: "problem", problem: cameraProblem(error)})
            }
        }
        void open()
        return () => {
            active = false
            if (opened !== null) stopStream(opened)
            setStream(null)
        }
    }, [needsCamera, facing, services])

    useEffect(() => {
        if (state.justCaptured === null) return
        const timer = setTimeout(() => dispatch({type: "flashDone"}), FLASH_MS)
        return () => clearTimeout(timer)
    }, [state.justCaptured])

    useEffect(() => {
        if (state.phase !== Phase.Capturing) return
        const timer = setInterval(() => dispatch({type: "tick", at: performance.now()}), TICK_MS)
        return () => clearInterval(timer)
    }, [state.phase])

    useEffect(() => {
        const form = formRef.current
        if (state.phase !== Phase.Checking || form === null) return
        populateCaptureForm(form, state.captures)
        // Safari before 16 has no requestSubmit.
        if ("requestSubmit" in form) {
            form.requestSubmit()
        } else {
            HTMLFormElement.prototype.submit.call(form)
        }
    }, [state.phase, state.captures])

    const onCaptured = (captured: CaptureStep, blob: Blob) => {
        services.vibrate(60)
        dispatch({type: "captured", step: captured, blob, at: performance.now()})
    }
    const onProblem = (next: CaptureProblem) => dispatch({type: "problem", problem: next})
    const start = () => {
        // Only our own face video needs a recorder.
        if (liveness !== undefined || services.recorder.supported()) {
            dispatch({type: "start"})
        } else {
            dispatch({type: "problem", problem: CaptureProblem.RecorderUnsupported})
        }
    }
    const retry = () => {
        if (problem === CaptureProblem.LivenessExpired) {
            // A GET of the page renders it again with a new token for the iframe.
            window.location.assign(window.location.href)
            return
        }
        if (problem === CaptureProblem.AnalyzerFailed) {
            setAnalyzerFailed(false)
            setAnalyzerAttempt((attempt) => attempt + 1)
        }
        dispatch({type: "retry"})
    }
    const verifyFrame = enrollmentFrame(kcContext, i18n, EnrollmentStep.VerifyIdentity)
    const card = {
        kcContext,
        i18n,
        doUseDefaultCss,
        classes,
        ...verifyFrame,
        displayMessage: state.phase === Phase.Intro,
    }

    if (problem !== null) {
        const [titleKey, bodyKey] = PROBLEMS[problem]
        const title = text(titleKey)
        const body = text(bodyKey)
        const canRetry = problem !== CaptureProblem.RecorderUnsupported
        const retryLabel = text(
            problem === CaptureProblem.LivenessExpired ? "scanovateStartOver" : "scanovateTryAgain"
        )
        return (
            <Template
                {...card}
                symbol={<CameraOffIcon />}
                symbolTone={SymbolTone.Warning}
                headerNode={title.text}
                titleLang={title.lang}
            >
                <p className="auth-lead" lang={body.lang}>
                    {body.text}
                </p>
                <Box className="auth-actions">
                    {canRetry && (
                        <Button
                            variant="contained"
                            fullWidth
                            className="auth-submit"
                            endIcon={<ArrowIcon />}
                            onClick={retry}
                            lang={retryLabel.lang}
                        >
                            {retryLabel.text}
                        </Button>
                    )}
                    <Button
                        variant="outlined"
                        fullWidth
                        onClick={() => dispatch({type: "confirmStop"})}
                        lang={text("scanovateBackToStart").lang}
                    >
                        {text("scanovateBackToStart").text}
                    </Button>
                </Box>
            </Template>
        )
    }

    if (state.phase === Phase.Intro) {
        return (
            <Template
                {...card}
                symbol={<FaceScanIcon />}
                headerNode={text("scanovateIntroTitle").text}
                titleLang={text("scanovateIntroTitle").lang}
            >
                <Intro
                    text={text}
                    document={document}
                    bothSides={scanovate.sides.includes(ScanovateSide.Back)}
                    liveness={liveness !== undefined}
                    attemptsLeft={
                        scanovate.attemptsLeft < scanovate.maxAttempts
                            ? scanovate.attemptsLeft
                            : null
                    }
                    onStart={start}
                />
            </Template>
        )
    }

    if (state.phase === Phase.Checking) {
        return (
            <Template
                {...card}
                symbol={<span className="spin" />}
                headerNode={text("scanovateCheckingTitle").text}
                titleLang={text("scanovateCheckingTitle").lang}
            >
                <Checking text={text} liveness={liveness !== undefined} />
                <form
                    ref={formRef}
                    method="post"
                    encType="multipart/form-data"
                    action={url.loginAction}
                    hidden
                >
                    <input type="hidden" name="action" value={CAPTURE_ACTION} />
                    {MEDIA_STEPS.map((part) => (
                        <input
                            key={part}
                            type="file"
                            name={CAPTURE_PARTS[part]}
                            tabIndex={-1}
                            aria-hidden="true"
                        />
                    ))}
                </form>
            </Template>
        )
    }

    if (state.phase === Phase.Liveness && liveness !== undefined) {
        const heading = text("scanovateCaptureLivenessHeading")
        const body = text("scanovateCaptureLivenessText")
        const stop = text("scanovateStop")
        return (
            <Template
                {...card}
                layout={phone ? TemplateLayout.Fullscreen : TemplateLayout.Wide}
                headerNode={text(STEP_TITLES[CaptureStep.Liveness]).text}
            >
                <section
                    className={phone ? "capture capture-liveness-page" : "capture-liveness-page"}
                >
                    {!phone && (
                        <header className="capture-liveness-header">
                            <h2 lang={heading.lang}>{heading.text}</h2>
                            <p lang={body.lang}>{body.text}</p>
                        </header>
                    )}
                    <LivenessFrame
                        liveness={liveness}
                        attempt={state.livenessAttempt}
                        languageTag={i18n.currentLanguage.languageTag}
                        title={text("scanovateLivenessFrameTitle").text}
                        onDone={() => dispatch({type: "livenessDone"})}
                        onProblem={(next) =>
                            next === null
                                ? dispatch({type: "confirmStop"})
                                : dispatch({type: "problem", problem: next})
                        }
                    />
                    {!phone && (
                        <Button
                            variant="outlined"
                            fullWidth
                            onClick={() => dispatch({type: "requestStop"})}
                            lang={stop.lang}
                        >
                            {stop.text}
                        </Button>
                    )}
                </section>
                <StopDialog
                    open={state.stopConfirmOpen}
                    text={text}
                    onCancel={() => dispatch({type: "cancelStop"})}
                    onConfirm={() => dispatch({type: "confirmStop"})}
                />
            </Template>
        )
    }

    const helpTitle = text("scanovateHelpTitle")
    return (
        <Template
            {...card}
            layout={phone ? TemplateLayout.Fullscreen : TemplateLayout.Wide}
            headerNode={text(STEP_TITLES[step]).text}
        >
            <CaptureCamera
                step={step}
                steps={steps}
                stream={stream}
                mirrored={facing === CameraFacing.User}
                analyzers={analyzers}
                recorder={services.recorder}
                videoSeconds={scanovate.videoSeconds}
                layout={phone ? StageLayout.Phone : StageLayout.Desktop}
                paused={state.helpOpen || state.stopConfirmOpen}
                flash={state.justCaptured !== null}
                text={text}
                documentLabel={document}
                eyebrow={verifyFrame.eyebrow ?? text("scanovateStepVerifyIdentity")}
                onCaptured={onCaptured}
                onProblem={onProblem}
                onHelp={() => dispatch({type: "openHelp"})}
                onStop={() => dispatch({type: "requestStop"})}
            />
            <Dialog
                open={state.helpOpen}
                onClose={() => dispatch({type: "closeHelp"})}
                className="capture-dialog"
                aria-labelledby="capture-help-title"
                fullWidth
                maxWidth="xs"
            >
                <DialogTitle id="capture-help-title" lang={helpTitle.lang}>
                    {helpTitle.text}
                </DialogTitle>
                <DialogContent>
                    <CheckList items={HELP[step].map((key) => text(key))} />
                </DialogContent>
                <DialogActions>
                    <Button
                        variant="contained"
                        fullWidth
                        onClick={() => dispatch({type: "closeHelp"})}
                        lang={text("scanovateHelpClose").lang}
                    >
                        {text("scanovateHelpClose").text}
                    </Button>
                </DialogActions>
            </Dialog>
            <StopDialog
                open={state.stopConfirmOpen}
                text={text}
                onCancel={() => dispatch({type: "cancelStop"})}
                onConfirm={() => dispatch({type: "confirmStop"})}
            />
        </Template>
    )
}

function StopDialog(props: {
    open: boolean
    text: Text
    onCancel: () => void
    onConfirm: () => void
}) {
    const {open, text, onCancel, onConfirm} = props
    return (
        <Dialog
            open={open}
            onClose={onCancel}
            className="capture-dialog"
            aria-labelledby="capture-stop-title"
            aria-describedby="capture-stop-text"
            fullWidth
            maxWidth="xs"
        >
            <DialogTitle id="capture-stop-title" lang={text("scanovateStopTitle").lang}>
                {text("scanovateStopTitle").text}
            </DialogTitle>
            <DialogContent>
                <p id="capture-stop-text" lang={text("scanovateStopText").lang}>
                    {text("scanovateStopText").text}
                </p>
            </DialogContent>
            <DialogActions>
                <Button
                    variant="outlined"
                    fullWidth
                    onClick={onCancel}
                    lang={text("scanovateStopCancel").lang}
                >
                    {text("scanovateStopCancel").text}
                </Button>
                <Button
                    variant="contained"
                    fullWidth
                    onClick={onConfirm}
                    lang={text("scanovateStopConfirm").lang}
                >
                    {text("scanovateStopConfirm").text}
                </Button>
            </DialogActions>
        </Dialog>
    )
}

function Intro(props: {
    text: Text
    document: TemplateLabel
    bothSides: boolean
    liveness: boolean
    attemptsLeft: number | null
    onStart: () => void
}) {
    const {text, document, bothSides, liveness, attemptsLeft, onStart} = props
    const lead = text(liveness ? "scanovateIntroLeadLiveness" : "scanovateIntroLead", document.text)
    type Item = [() => ReactNode, MessageKey, MessageKey]
    const face: Item[] = liveness
        ? [[FaceIcon, "scanovateIntroFace", "scanovateIntroLivenessHint"]]
        : [
              [FaceIcon, "scanovateIntroFace", "scanovateIntroFaceHint"],
              [VideoIcon, "scanovateIntroVideo", "scanovateIntroVideoHint"],
          ]
    const items: Item[] = [
        [
            IdCardIcon,
            bothSides ? "scanovateIntroDocumentBothSides" : "scanovateIntroDocumentFrontSide",
            "scanovateIntroDocumentHint",
        ],
        ...face,
    ]
    const before = text("scanovateIntroBeforeTitle")
    const privacy = text("scanovateIntroPrivacy")
    const start = text("scanovateIntroStart")
    return (
        <>
            <p className="auth-lead" lang={lead.lang}>
                {lead.text}
            </p>
            <ol className="auth-steps">
                {items.map(([Icon, titleKey, hintKey]) => {
                    const title = text(titleKey)
                    const hint = text(hintKey)
                    return (
                        <li key={titleKey}>
                            <span className="num" aria-hidden="true">
                                <Icon />
                            </span>
                            <div>
                                <strong lang={title.lang}>{title.text}</strong>
                                <span lang={hint.lang}>{hint.text}</span>
                            </div>
                        </li>
                    )
                })}
            </ol>
            <section className="auth-panel" aria-labelledby="scanovate-before">
                <h2 id="scanovate-before" className="auth-panel-title" lang={before.lang}>
                    {before.text}
                </h2>
                <CheckList
                    items={[
                        text("scanovateIntroTipLight"),
                        text("scanovateIntroTipCoverings"),
                        text("scanovateIntroTipDocument", document.text),
                    ]}
                />
            </section>
            <p className="auth-privacy" lang={privacy.lang}>
                <LockIcon />
                <span>{privacy.text}</span>
            </p>
            <Box className="auth-actions">
                <Button
                    variant="contained"
                    fullWidth
                    className="auth-submit"
                    endIcon={<ArrowIcon />}
                    onClick={onStart}
                    lang={start.lang}
                >
                    {start.text}
                </Button>
                {attemptsLeft !== null && <AttemptsLeft text={text} attemptsLeft={attemptsLeft} />}
            </Box>
        </>
    )
}

function Checking({text, liveness}: {text: Text; liveness: boolean}) {
    const lead = text("scanovateCheckingLead")
    const steps: [MessageKey, string][] = [
        [liveness ? "scanovateCheckingReceivedLiveness" : "scanovateCheckingReceived", "done"],
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
