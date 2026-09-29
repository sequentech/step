// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, useLayoutEffect, useRef, useState, type ReactNode} from "react"
import Button from "@mui/material/Button"
import Typography from "@mui/material/Typography"
import type {TemplateLabel} from "../Template"
import {
    CheckCircleIcon,
    CheckIcon,
    CloseIcon,
    FaceIcon,
    HelpIcon,
    IdCardIcon,
    LightIcon,
    LockIcon,
    VideoIcon,
} from "../icons"
import type {MessageKey} from "../i18n"
import {
    DOCUMENT_ANALYSIS_SIDE,
    FACE_ANALYSIS_SIDE,
    FrameSampler,
    grabStill,
    videoSize,
} from "./frames"
import {
    PILL_HEIGHT,
    RING_GAP,
    StageLayout,
    ellipseToFrame,
    overlayFor,
    rectToFrame,
    type Overlay,
} from "./geometry"
import {
    Tone,
    canTakePhoto,
    documentGood,
    documentGuidance,
    documentVisible,
    faceGood,
    faceGuidance,
    videoGuidance,
    type Guidance,
} from "./guidance"
import {useElementSize, useThrottledAnnouncement} from "./hooks"
import {capitalized, type Text} from "./text"
import {
    CaptureProblem,
    CaptureStep,
    DocumentStatus,
    FaceStatus,
    type Analyzers,
    type Ellipse,
    type Rect,
    type RecorderService,
    type Recording,
    type Size,
} from "./types"
import {VideoCommand, idleVideo, trackVideo} from "./video"

// About 15 analyses per second.
const ANALYSIS_INTERVAL_MS = 66
// Nothing is captured right after a step starts: the success flash is showing
// and, for the back, the voter has not turned the ID over yet.
const STEP_SETTLE_MS = 900
const ANNOUNCE_INTERVAL_MS = 3000
const GREEN = "#43e3a1"
// Room above the oval for the REC badge on larger screens.
const DESKTOP_REC_SPACE = 60

export const STEP_TITLES: Record<CaptureStep, MessageKey> = {
    [CaptureStep.Front]: "scanovateCaptureFrontTitle",
    [CaptureStep.Back]: "scanovateCaptureBackTitle",
    [CaptureStep.Face]: "scanovateCaptureFaceTitle",
    [CaptureStep.Video]: "scanovateCaptureVideoTitle",
    [CaptureStep.Liveness]: "scanovateCaptureLivenessTitle",
}

const STEP_HEADINGS: Record<CaptureStep, [MessageKey, MessageKey]> = {
    [CaptureStep.Front]: ["scanovateCaptureFrontHeading", "scanovateCaptureFrontText"],
    [CaptureStep.Back]: ["scanovateCaptureBackHeading", "scanovateCaptureBackText"],
    [CaptureStep.Face]: ["scanovateCaptureFaceHeading", "scanovateCaptureFaceText"],
    [CaptureStep.Video]: ["scanovateCaptureVideoHeading", "scanovateCaptureVideoText"],
    [CaptureStep.Liveness]: ["scanovateCaptureLivenessHeading", "scanovateCaptureLivenessText"],
}

const CHIPS: Partial<Record<CaptureStep, MessageKey>> = {
    [CaptureStep.Front]: "scanovateChipFront",
    [CaptureStep.Back]: "scanovateChipBack",
    [CaptureStep.Face]: "scanovateChipFace",
}

const STEP_ICONS: Record<CaptureStep, () => ReactNode> = {
    [CaptureStep.Front]: IdCardIcon,
    [CaptureStep.Back]: IdCardIcon,
    [CaptureStep.Face]: FaceIcon,
    [CaptureStep.Video]: VideoIcon,
    [CaptureStep.Liveness]: FaceIcon,
}

type Live = {
    document: DocumentStatus | null
    face: FaceStatus | null
    progress: number
    recordingSeconds: number | null
}

const WAITING: Live = {document: null, face: null, progress: 0, recordingSeconds: null}

function sameLive(a: Live, b: Live): boolean {
    return (
        a.document === b.document &&
        a.face === b.face &&
        Math.abs(a.progress - b.progress) < 0.02 &&
        a.recordingSeconds === b.recordingSeconds
    )
}

function isDocumentStep(step: CaptureStep): boolean {
    return step === CaptureStep.Front || step === CaptureStep.Back
}

export function guidanceFor(step: CaptureStep, live: Live): Guidance {
    switch (step) {
        case CaptureStep.Front:
        case CaptureStep.Back:
            return documentGuidance(step, live.document)
        case CaptureStep.Face:
        case CaptureStep.Liveness:
            return faceGuidance(live.face)
        case CaptureStep.Video:
            return videoGuidance(live.face, live.document, live.recordingSeconds !== null)
    }
}

export function formatElapsed(seconds: number): string {
    return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`
}

export type CaptureCameraProps = {
    step: CaptureStep
    steps: CaptureStep[]
    stream: MediaStream | null
    mirrored: boolean
    analyzers: Analyzers | null
    recorder: RecorderService
    videoSeconds: number
    layout: StageLayout
    paused: boolean
    flash: boolean
    text: Text
    documentLabel: TemplateLabel
    eyebrow: TemplateLabel
    onCaptured: (step: CaptureStep, blob: Blob) => void
    onProblem: (problem: CaptureProblem) => void
    onHelp: () => void
    onStop: () => void
}

type LoopInput = {
    stage: Size | null
    overlay: Overlay | null
    mirrored: boolean
    paused: boolean
    videoSeconds: number
    onCaptured: (step: CaptureStep, blob: Blob) => void
    onProblem: (problem: CaptureProblem) => void
}

export default function CaptureCamera(props: CaptureCameraProps) {
    const {
        step,
        steps,
        stream,
        mirrored,
        analyzers,
        recorder,
        videoSeconds,
        layout,
        paused,
        flash,
        text,
        documentLabel,
        eyebrow,
        onCaptured,
        onProblem,
        onHelp,
        onStop,
    } = props
    const videoRef = useRef<HTMLVideoElement | null>(null)
    const stageRef = useRef<HTMLDivElement | null>(null)
    const topRef = useRef<HTMLDivElement | null>(null)
    const sheetRef = useRef<HTMLElement | null>(null)
    const stage = useElementSize(stageRef)
    const top = useElementSize(topRef)
    const sheet = useElementSize(sheetRef)
    const [live, setLive] = useState<Live>(WAITING)
    const [photoStep, setPhotoStep] = useState<CaptureStep | null>(null)
    const phone = layout === StageLayout.Phone

    const overlay =
        stage === null
            ? null
            : overlayFor(
                  step,
                  stage,
                  phone
                      ? {top: (top?.height ?? 0) + 16, bottom: (sheet?.height ?? 0) + 8}
                      : {top: step === CaptureStep.Video ? DESKTOP_REC_SPACE : 24, bottom: 0},
                  layout
              )

    const input = useRef<LoopInput | null>(null)
    useLayoutEffect(() => {
        input.current = {stage, overlay, mirrored, paused, videoSeconds, onCaptured, onProblem}
    })

    useEffect(() => {
        const video = videoRef.current
        if (video === null || stream === null) return
        video.srcObject = stream
        const play = async () => {
            try {
                await video.play()
            } catch {
                // Autoplay of a muted inline video only fails when the element is detached.
            }
        }
        void play()
        return () => {
            video.srcObject = null
        }
    }, [stream])

    useEffect(() => {
        const video = videoRef.current
        if (video === null || stream === null || analyzers === null) return
        const documentSampler = new FrameSampler()
        const faceSampler = new FrameSampler()
        const startedAt = performance.now()
        let frameId = 0
        let last = 0
        let tick = 0
        let busy = false
        // The back is only taken once the front has left the frame.
        let armed = step !== CaptureStep.Back
        let tracker = idleVideo
        let recording: Recording | null = null
        let documentStatus: DocumentStatus | null = null
        let current = WAITING
        analyzers.document.reset()
        analyzers.face.reset()

        const publish = (next: Live) => {
            if (!sameLive(current, next)) {
                current = next
                setLive(next)
            }
        }
        const capture = async () => {
            busy = true
            try {
                const blob = await grabStill(video)
                input.current?.onCaptured(step, blob)
            } catch {
                busy = false
            }
        }
        const finish = async (finished: Recording) => {
            busy = true
            try {
                const blob = await finished.stop()
                input.current?.onCaptured(step, blob)
            } catch {
                busy = false
                analyzers.face.reset()
            }
        }

        const analyzeDocument = (guide: Rect, stageSize: Size, size: Size, flip: boolean) => {
            const image = documentSampler.sample(video, DOCUMENT_ANALYSIS_SIDE)
            if (image === null) return null
            const frame = rectToFrame(guide, stageSize, size, flip, image)
            return analyzers.document.analyze(
                image.data,
                image.width,
                image.height,
                frame.x,
                frame.y,
                frame.width,
                frame.height
            )
        }
        const analyzeFace = (oval: Ellipse, stageSize: Size, size: Size, flip: boolean) => {
            const image = faceSampler.sample(video, FACE_ANALYSIS_SIDE)
            if (image === null) return null
            const frame = ellipseToFrame(oval, stageSize, size, flip, image)
            return analyzers.face.analyze(
                image.data,
                image.width,
                image.height,
                frame.cx,
                frame.cy,
                frame.rx,
                frame.ry
            )
        }

        const loop = (now: number) => {
            frameId = requestAnimationFrame(loop)
            const state = input.current
            const size = videoSize(video)
            if (
                busy ||
                now - last < ANALYSIS_INTERVAL_MS ||
                state === null ||
                state.stage === null ||
                state.overlay === null ||
                size === null
            ) {
                return
            }
            last = now
            tick += 1
            const {stage: stageSize, overlay: shape, mirrored: flip, paused: halted} = state
            const settled = now - startedAt >= STEP_SETTLE_MS

            if (shape.guide !== undefined) {
                const result = analyzeDocument(shape.guide, stageSize, size, flip)
                if (result === null) return
                armed ||= !documentGood(result.status)
                publish({...WAITING, document: result.status, progress: result.stability})
                if (result.status === DocumentStatus.Ready && armed && settled && !halted) {
                    void capture()
                }
                return
            }
            if (shape.oval === undefined) return
            const face = analyzeFace(shape.oval, stageSize, size, flip)
            if (face === null) return
            if (shape.bracket === undefined) {
                publish({...WAITING, face: face.status, progress: face.stability})
                if (face.status === FaceStatus.Ready && settled && !halted) {
                    void capture()
                }
                return
            }
            if (tick % 2 === 1 || documentStatus === null) {
                documentStatus =
                    analyzeDocument(shape.bracket, stageSize, size, flip)?.status ?? null
            }
            const seconds = state.videoSeconds
            if (halted || !settled) {
                recording?.cancel()
                recording = null
                tracker = idleVideo
            } else {
                const next = trackVideo(tracker, {
                    faceReady: face.status === FaceStatus.Ready,
                    faceInView: faceGood(face.status),
                    documentInView: documentVisible(documentStatus),
                    now,
                    seconds,
                })
                tracker = next.tracker
                if (next.command === VideoCommand.Start) {
                    try {
                        recording = recorder.start(stream, seconds)
                    } catch {
                        state.onProblem(CaptureProblem.RecorderUnsupported)
                        return
                    }
                } else if (next.command === VideoCommand.Restart) {
                    recording?.cancel()
                    recording = null
                    analyzers.face.reset()
                } else if (next.command === VideoCommand.Finish && recording !== null) {
                    const finished = recording
                    recording = null
                    publish({...current, recordingSeconds: seconds})
                    void finish(finished)
                    return
                }
            }
            const since = tracker.recordingSince
            publish({
                document: documentStatus,
                face: face.status,
                progress: since === null ? 0 : Math.min((now - since) / (seconds * 1000), 1),
                recordingSeconds: since === null ? null : Math.floor((now - since) / 1000),
            })
        }
        frameId = requestAnimationFrame(loop)
        return () => {
            cancelAnimationFrame(frameId)
            recording?.cancel()
            setLive(WAITING)
        }
    }, [stream, step, analyzers, recorder])

    const guidance: Guidance =
        stream === null
            ? {message: "scanovateCameraStarting", tone: Tone.Guide}
            : guidanceFor(step, live)
    const guidanceText = text(guidance.message)
    const announcement = useThrottledAnnouncement(
        flash ? text("scanovateCaptured").text : guidanceText.text,
        ANNOUNCE_INTERVAL_MS
    )
    const stepIndex = steps.indexOf(step)
    const title = text(STEP_TITLES[step])
    const [headingKey, bodyKey] = STEP_HEADINGS[step]
    const heading = text(headingKey)
    const body = text(bodyKey)
    const counter = text(
        "scanovateCaptureCounter",
        capitalized(documentLabel).text,
        String(stepIndex + 1),
        String(steps.length)
    )
    const status = step === CaptureStep.Face ? live.face : live.document
    const shutterEnabled =
        step !== CaptureStep.Video && canTakePhoto(status) && photoStep !== step && !paused
    const takePhoto = async () => {
        const video = videoRef.current
        if (video === null) return
        setPhotoStep(step)
        try {
            const blob = await grabStill(video)
            onCaptured(step, blob)
        } catch {
            setPhotoStep(null)
        }
    }
    const chips = steps
        .slice(0, stepIndex)
        .map((done) => CHIPS[done])
        .filter((key): key is MessageKey => key !== undefined)
        .map((key) => text(key))

    const stageNode = (
        <div className="capture-stage" ref={stageRef}>
            <video
                ref={videoRef}
                className={mirrored ? "capture-video mirrored" : "capture-video"}
                autoPlay
                muted
                playsInline
                aria-hidden="true"
            />
            {stage !== null && overlay !== null && (
                <StageOverlay
                    stage={stage}
                    overlay={overlay}
                    step={step}
                    live={live}
                    dim={phone ? 1 : 0.85}
                />
            )}
            {live.recordingSeconds !== null && (
                <div
                    className="capture-rec"
                    style={{top: phone ? (top?.height ?? 0) + 24 : 16}}
                    lang={text("scanovateRecording").lang}
                >
                    <i aria-hidden="true" />
                    {text("scanovateRecording", formatElapsed(live.recordingSeconds)).text}
                </div>
            )}
            {overlay !== null && (
                <div
                    className={guidance.tone === Tone.Ok ? "capture-pill ok" : "capture-pill"}
                    style={{top: overlay.pillTop, minHeight: PILL_HEIGHT}}
                    lang={guidanceText.lang}
                    aria-hidden="true"
                >
                    {guidance.tone === Tone.Ok ? <CheckCircleIcon /> : <LightIcon />}
                    <span>{guidanceText.text}</span>
                </div>
            )}
            {flash && <div className="capture-flash" aria-hidden="true" />}
        </div>
    )
    const liveRegion = (
        <div className="capture-announcer" aria-live="polite" aria-atomic="true">
            {announcement}
        </div>
    )

    if (!phone) {
        return (
            <div className="desk-capture">
                <div className="desk-cam">{stageNode}</div>
                <div className="desk-side">
                    <Typography className="auth-eyebrow" lang={eyebrow.lang}>
                        {eyebrow.text}
                    </Typography>
                    <Typography
                        id="kc-page-title"
                        component="h1"
                        className="auth-title"
                        lang={title.lang}
                    >
                        {title.text}
                    </Typography>
                    <div className="desk-instructions">
                        <h2 lang={heading.lang}>{heading.text}</h2>
                        <p lang={body.lang}>{body.text}</p>
                    </div>
                    <ol className="desk-steps">
                        {steps.map((each, index) => {
                            const Icon = STEP_ICONS[each]
                            const label = text(STEP_TITLES[each])
                            const state =
                                index < stepIndex ? "done" : index === stepIndex ? "current" : ""
                            return (
                                <li
                                    key={each}
                                    className={state}
                                    lang={label.lang}
                                    aria-current={index === stepIndex ? "step" : undefined}
                                >
                                    {index < stepIndex ? (
                                        <CheckIcon />
                                    ) : index === stepIndex ? (
                                        <span className="spin" aria-hidden="true" />
                                    ) : (
                                        <Icon />
                                    )}
                                    {label.text}
                                </li>
                            )
                        })}
                    </ol>
                    <div className="desk-actions">
                        {step !== CaptureStep.Video && (
                            <Button
                                variant="contained"
                                fullWidth
                                disabled={!shutterEnabled}
                                onClick={() => void takePhoto()}
                                lang={text("scanovateTakePhoto").lang}
                            >
                                {text("scanovateTakePhoto").text}
                            </Button>
                        )}
                        <div className="desk-secondary">
                            <Button
                                variant="text"
                                onClick={onHelp}
                                startIcon={<HelpIcon />}
                                lang={text("scanovateHelp").lang}
                            >
                                {text("scanovateHelp").text}
                            </Button>
                            <Button
                                variant="text"
                                onClick={onStop}
                                startIcon={<CloseIcon />}
                                lang={text("scanovateStop").lang}
                            >
                                {text("scanovateStop").text}
                            </Button>
                        </div>
                    </div>
                </div>
                {liveRegion}
            </div>
        )
    }

    return (
        <div className="capture">
            {stageNode}
            <div className="capture-topbar" ref={topRef}>
                <button
                    type="button"
                    onClick={onStop}
                    aria-label={text("scanovateStop").text}
                    lang={text("scanovateStop").lang}
                >
                    <CloseIcon />
                </button>
                <div className="capture-title">
                    <h1 id="kc-page-title" lang={title.lang}>
                        {title.text}
                    </h1>
                    <span lang={counter.lang}>{counter.text}</span>
                    <div className="capture-dots" aria-hidden="true">
                        {steps.map((each, index) => (
                            <i
                                key={each}
                                className={
                                    index < stepIndex
                                        ? "done"
                                        : index === stepIndex
                                          ? "current"
                                          : undefined
                                }
                            />
                        ))}
                    </div>
                </div>
                <button
                    type="button"
                    onClick={onHelp}
                    aria-label={text("scanovateHelp").text}
                    lang={text("scanovateHelp").lang}
                >
                    <HelpIcon />
                </button>
            </div>
            <section className="capture-sheet" ref={sheetRef} aria-labelledby="capture-heading">
                <h2 id="capture-heading" lang={heading.lang}>
                    {heading.text}
                </h2>
                <p lang={body.lang}>{body.text}</p>
                {(chips.length > 0 || isDocumentStep(step)) && (
                    <div className="capture-row">
                        <ul className="capture-chips">
                            {chips.map((chip) => (
                                <li key={chip.text} lang={chip.lang}>
                                    <CheckIcon />
                                    {chip.text}
                                </li>
                            ))}
                        </ul>
                        {isDocumentStep(step) && (
                            <button
                                type="button"
                                className="capture-shutter"
                                disabled={!shutterEnabled}
                                onClick={() => void takePhoto()}
                                aria-label={text("scanovateTakePhoto").text}
                                lang={text("scanovateTakePhoto").lang}
                            />
                        )}
                    </div>
                )}
                <div className="capture-secure" lang={text("scanovateCaptureSecure").lang}>
                    <LockIcon />
                    {text("scanovateCaptureSecure").text}
                </div>
            </section>
            {liveRegion}
        </div>
    )
}

function roundedRect({x, y, width, height}: Rect, radius: number): string {
    const r = Math.min(radius, width / 2, height / 2)
    return (
        `M${x} ${y + r} V${y + height - r} Q${x} ${y + height} ${x + r} ${y + height} ` +
        `H${x + width - r} Q${x + width} ${y + height} ${x + width} ${y + height - r} ` +
        `V${y + r} Q${x + width} ${y} ${x + width - r} ${y} H${x + r} Q${x} ${y} ${x} ${y + r} Z`
    )
}

function ellipsePath({cx, cy, rx, ry}: Ellipse): string {
    return `M${cx - rx} ${cy} A${rx} ${ry} 0 1 0 ${cx + rx} ${cy} A${rx} ${ry} 0 1 0 ${cx - rx} ${cy} Z`
}

function Brackets({rect, color}: {rect: Rect; color: string}) {
    const {x, y, width: w, height: h} = rect
    const arm = Math.min(34, w / 3, h / 3)
    return (
        <g stroke={color} strokeWidth="5" strokeLinecap="round" fill="none">
            <path d={`M${x} ${y + arm} V${y + 10} Q${x} ${y} ${x + 10} ${y} H${x + arm}`} />
            <path
                d={`M${x + w - arm} ${y} H${x + w - 10} Q${x + w} ${y} ${x + w} ${y + 10} V${y + arm}`}
            />
            <path
                d={`M${x} ${y + h - arm} V${y + h - 10} Q${x} ${y + h} ${x + 10} ${y + h} H${x + arm}`}
            />
            <path
                d={`M${x + w - arm} ${y + h} H${x + w - 10} Q${x + w} ${y + h} ${x + w} ${y + h - 10} V${y + h - arm}`}
            />
        </g>
    )
}

function StageOverlay(props: {
    stage: Size
    overlay: Overlay
    step: CaptureStep
    live: Live
    dim: number
}) {
    const {stage, overlay, step, live, dim} = props
    const outside = `M0 0 H${stage.width} V${stage.height} H0 Z`
    if (overlay.guide !== undefined) {
        const good = documentGood(live.document)
        return (
            <svg
                className="capture-overlay"
                viewBox={`0 0 ${stage.width} ${stage.height}`}
                aria-hidden="true"
            >
                <path
                    d={`${outside} ${roundedRect(overlay.guide, 16)}`}
                    fill="#0b2231"
                    opacity={0.58 * dim}
                    fillRule="evenodd"
                />
                <Brackets rect={overlay.guide} color={good ? GREEN : "#ffffff"} />
            </svg>
        )
    }
    if (overlay.oval === undefined) return null
    const {cx, cy, rx, ry} = overlay.oval
    const ring = {cx, cy, rx: rx + RING_GAP, ry: ry + RING_GAP}
    const video = step === CaptureStep.Video
    return (
        <svg
            className="capture-overlay"
            viewBox={`0 0 ${stage.width} ${stage.height}`}
            aria-hidden="true"
        >
            <path
                d={`${outside} ${ellipsePath(overlay.oval)}`}
                fill="#0b2231"
                opacity={(video ? 0.35 : 0.72) * dim}
                fillRule="evenodd"
            />
            <ellipse
                cx={ring.cx}
                cy={ring.cy}
                rx={ring.rx}
                ry={ring.ry}
                fill="none"
                stroke="#ffffff33"
                strokeWidth="6"
            />
            {live.progress > 0 && (
                <path
                    className="capture-ring"
                    d={`M${cx} ${cy - ring.ry} A${ring.rx} ${ring.ry} 0 1 1 ${cx} ${cy + ring.ry} A${ring.rx} ${ring.ry} 0 1 1 ${cx} ${cy - ring.ry}`}
                    pathLength={100}
                    fill="none"
                    stroke={GREEN}
                    strokeWidth="6"
                    strokeLinecap="round"
                    strokeDasharray={`${live.progress * 100} 100`}
                />
            )}
            {overlay.bracket !== undefined && (
                <Brackets
                    rect={overlay.bracket}
                    color={documentVisible(live.document) ? GREEN : "#ffffff"}
                />
            )}
        </svg>
    )
}
