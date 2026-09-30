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
import {CAPTURE_ACTION, captureParts} from "../scanovate/form"
import {uploadCaptures} from "../scanovate/uploads"
import {StageLayout} from "../scanovate/geometry"
import {Tone, type Guidance} from "../scanovate/guidance"
import {LivenessCheck, LivenessOutcomeKind} from "../scanovate/liveness"
import {LivenessAbort} from "../scanovate/livenessApi"
import {stopStream, cameraProblem} from "../scanovate/media"
import type {ScanovatePageProps} from "../scanovate/pageProps"
import {EnrollmentStep, documentName, enrollmentFrame, textFor, type Text} from "../scanovate/text"
import {
    CameraFacing,
    CaptureProblem,
    CaptureStep,
    FaceCheck,
    VideoOutput,
    type Analyzers,
} from "../scanovate/types"

const FLASH_MS = 700
// How long the advice of Liveness Plus on a rejected face frame stays.
const HINT_MS = 3000
const CHECKING_FACE: Guidance = {message: "scanovateLivenessChecking", tone: Tone.Ok}
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
    [CaptureProblem.UploadFailed]: ["scanovateUploadFailedTitle", "scanovateUploadFailedText"],
    [CaptureProblem.CaptureExpired]: [
        "scanovateCaptureExpiredTitle",
        "scanovateCaptureExpiredText",
    ],
}

// Problems that only a new page, with new one-time tokens, can get past.
const STALE_PAGE = new Set<CaptureProblem | null>([
    CaptureProblem.LivenessExpired,
    CaptureProblem.CaptureExpired,
])

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
}

export default function ScanovateCapture(props: ScanovatePageProps<"scanovate-capture.ftl">) {
    const {kcContext, i18n, Template, doUseDefaultCss, classes} = props
    const {scanovate, url} = kcContext
    const {services, startAt} = useCaptureEnvironment()
    const text = textFor(kcContext, i18n)
    const phone = useMediaQuery(PHONE_QUERY, {noSsr: true})
    const liveness = scanovate.liveness
    const faceCheck = liveness === undefined ? FaceCheck.Photo : FaceCheck.Liveness
    const livenessCheck = useMemo(
        () => (liveness === undefined ? null : new LivenessCheck(services.liveness(liveness))),
        [services, liveness]
    )
    const uploader = useMemo(() => services.uploads(scanovate.upload), [services, scanovate.upload])
    const steps = useMemo(() => stepsFor(scanovate.sides), [scanovate.sides])
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
    // Liveness Plus checking a face frame, and its advice when it rejects one.
    const [checkingFace, setCheckingFace] = useState(false)
    const [faceHint, setFaceHint] = useState<Guidance | null>(null)
    const [faceAttempt, setFaceAttempt] = useState(0)
    // Bumped to ignore the answer to a face frame once the voter stopped.
    const faceRun = useRef(0)
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
        if (faceHint === null) return
        const timer = setTimeout(() => setFaceHint(null), HINT_MS)
        return () => clearTimeout(timer)
    }, [faceHint])

    // An open liveness session is ended when the voter leaves.
    useEffect(() => {
        if (livenessCheck === null) return
        const leave = () => livenessCheck.abort(LivenessAbort.LeftPage)
        window.addEventListener("pagehide", leave)
        return () => {
            window.removeEventListener("pagehide", leave)
            leave()
        }
    }, [livenessCheck])

    // Keycloak can't take files on its login actions URL: the captures are
    // uploaded first, and the form only carries the action.
    useEffect(() => {
        if (state.phase !== Phase.Checking) return
        let active = true
        const send = async () => {
            const failure = await uploadCaptures(uploader, captureParts(state.captures, faceCheck))
            const form = formRef.current
            if (!active || form === null) return
            if (failure !== null) {
                dispatch({type: "uploadFailed", problem: failure})
                return
            }
            // Safari before 16 has no requestSubmit.
            if ("requestSubmit" in form) {
                form.requestSubmit()
            } else {
                HTMLFormElement.prototype.submit.call(form)
            }
        }
        void send()
        return () => {
            active = false
        }
    }, [state.phase, state.captures, faceCheck, uploader])

    const accept = (captured: CaptureStep, blob: Blob) => {
        services.vibrate(60)
        dispatch({type: "captured", step: captured, blob, at: performance.now()})
    }
    // With liveness, the face frame only counts once Liveness Plus scanned it.
    const checkFace = async (check: LivenessCheck, blob: Blob) => {
        const run = faceRun.current
        setFaceHint(null)
        setCheckingFace(true)
        const outcome = await check.check(blob)
        if (run !== faceRun.current) return
        setCheckingFace(false)
        switch (outcome.kind) {
            case LivenessOutcomeKind.Completed:
                accept(CaptureStep.Face, blob)
                break
            case LivenessOutcomeKind.Retry:
                if (outcome.guidance !== null) {
                    setFaceHint({message: outcome.guidance, tone: Tone.Guide})
                }
                setFaceAttempt((attempt) => attempt + 1)
                break
            case LivenessOutcomeKind.Problem:
                dispatch({type: "problem", problem: outcome.problem})
                break
        }
    }
    const onCaptured = (captured: CaptureStep, blob: Blob) => {
        if (livenessCheck !== null && captured === CaptureStep.Face) {
            void checkFace(livenessCheck, blob)
            return
        }
        accept(captured, blob)
    }
    const onProblem = (next: CaptureProblem) => dispatch({type: "problem", problem: next})
    const stop = () => {
        faceRun.current += 1
        setCheckingFace(false)
        setFaceHint(null)
        livenessCheck?.abort(LivenessAbort.PressedClose)
        dispatch({type: "confirmStop"})
    }
    const start = () => {
        // Only our own face video needs a recorder.
        if (liveness !== undefined || services.recorder.supported()) {
            dispatch({type: "start"})
        } else {
            dispatch({type: "problem", problem: CaptureProblem.RecorderUnsupported})
        }
    }
    const retry = () => {
        if (STALE_PAGE.has(problem)) {
            // A GET of the page renders it again with new one-time tokens.
            window.location.assign(window.location.href)
            return
        }
        if (problem === CaptureProblem.AnalyzerFailed) {
            setAnalyzerFailed(false)
            setAnalyzerAttempt((attempt) => attempt + 1)
        }
        faceRun.current += 1
        setCheckingFace(false)
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
            STALE_PAGE.has(problem) ? "scanovateStartOver" : "scanovateTryAgain"
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
                        onClick={stop}
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
                <Checking text={text} faceCheck={faceCheck} />
                <form ref={formRef} method="post" action={url.loginAction} hidden>
                    <input type="hidden" name="action" value={CAPTURE_ACTION} />
                </form>
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
                videoOutput={
                    faceCheck === FaceCheck.Liveness ? VideoOutput.Still : VideoOutput.Recording
                }
                attempt={faceAttempt}
                hint={checkingFace ? CHECKING_FACE : faceHint}
                layout={phone ? StageLayout.Phone : StageLayout.Desktop}
                paused={state.helpOpen || state.stopConfirmOpen || checkingFace}
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
                onConfirm={stop}
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
    attemptsLeft: number | null
    onStart: () => void
}) {
    const {text, document, bothSides, attemptsLeft, onStart} = props
    const lead = text("scanovateIntroLead", document.text)
    type Item = [() => ReactNode, MessageKey, MessageKey]
    const items: Item[] = [
        [
            IdCardIcon,
            bothSides ? "scanovateIntroDocumentBothSides" : "scanovateIntroDocumentFrontSide",
            "scanovateIntroDocumentHint",
        ],
        [FaceIcon, "scanovateIntroFace", "scanovateIntroFaceHint"],
        [VideoIcon, "scanovateIntroVideo", "scanovateIntroVideoHint"],
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

const RECEIVED: Record<FaceCheck, MessageKey> = {
    [FaceCheck.Photo]: "scanovateCheckingReceived",
    [FaceCheck.Liveness]: "scanovateCheckingReceivedLiveness",
}

function Checking({text, faceCheck}: {text: Text; faceCheck: FaceCheck}) {
    const lead = text("scanovateCheckingLead")
    const steps: [MessageKey, string][] = [
        [RECEIVED[faceCheck], "done"],
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
