// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useCallback, useEffect, useRef, useState, type Dispatch} from "react"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import type {ScanovateHolding} from "../KcContext"
import {ArrowIcon, SwitchCameraIcon, CloseIcon} from "../icons"
import type {CaptureEvent} from "./capture"
import {
    CameraSession,
    HOLDING_CAMERA,
    SwitchOutcome,
    canSwitch,
    mirrorsPreview,
    type Camera,
    type OpenedCamera,
} from "./cameraSession"
import {takePhoto, uploadHoldingPhoto} from "./holding"
import {
    SAMPLE_HEIGHT,
    SAMPLE_WIDTH,
    STILL,
    grayscale,
    secondsLeft,
    trackStillness,
    type Stillness,
} from "./stillness"
import type {Text} from "./text"
import {CaptureProblem} from "./types"

type Photo = {blob: Blob; url: string}

/**
 * Whether the video shows a camera that is on. A camera stopped for a switch
 * leaves its last frame, perfectly still, and can still report it loaded:
 * no photo is ever taken of it.
 */
function showsLiveCamera(video: HTMLVideoElement): boolean {
    const stream = video.srcObject
    return (
        stream instanceof MediaStream &&
        stream.getVideoTracks().some((track) => track.readyState === "live")
    )
}

/**
 * Shows a camera, and opens the step's camera again if this one goes away
 * (unplugged, or taken by another app); `lost` when none opens. Stopping a
 * camera ourselves, to switch or leave, doesn't end it this way.
 */
function keepOpen(
    session: CameraSession,
    opened: OpenedCamera,
    display: (session: CameraSession, opened: OpenedCamera) => void,
    lost: () => void
) {
    display(session, opened)
    opened.stream.getVideoTracks()[0]?.addEventListener("ended", () => {
        if (session.camera !== opened.camera) return
        void session.reopen().then((again) => {
            if (again === null) {
                lost()
            } else {
                keepOpen(session, again, display, lost)
            }
        })
    })
}

// How often the camera image is sampled for movement.
const SAMPLE_INTERVAL_MS = 100

/**
 * The page's own camera: the voter holds their document next to their face,
 * the page takes the photo by itself once the image has been still for the
 * holding time, the voter checks it, and the page uploads it to Keycloak
 * before moving on. There is no shutter button: the photo is only ever taken
 * still.
 *
 * It looks like Scanovate's own pages, which come before it: the camera over
 * the whole capture window, a close button, and the instruction over the image.
 * When the browser gives more than one camera, a button switches between them,
 * front first (HOLDING_CAMERA); switching starts the holding time again.
 */
export function HoldingCamera(props: {
    text: Text
    holding: ScanovateHolding
    dispatch: Dispatch<CaptureEvent>
}) {
    const {text, holding, dispatch} = props
    const videoRef = useRef<HTMLVideoElement | null>(null)
    const [ready, setReady] = useState(false)
    const [progress, setProgress] = useState(0)
    const [photo, setPhoto] = useState<Photo | null>(null)
    const [sending, setSending] = useState(false)

    const sessionRef = useRef<CameraSession | null>(null)
    const [camera, setCamera] = useState<Camera | null>(null)
    const [cameras, setCameras] = useState<Camera[]>([])
    const [switching, setSwitching] = useState(false)
    const [switchFailed, setSwitchFailed] = useState(false)

    const refreshCameras = useCallback((session: CameraSession) => {
        void session.cameras().then((listed) => {
            if (sessionRef.current === session) setCameras(listed)
        })
    }, [])

    // Shows a camera that has just opened, and starts the holding time again.
    const display = useCallback(
        (session: CameraSession, opened: OpenedCamera) => {
            if (sessionRef.current !== session) return
            setReady(false)
            setProgress(0)
            setCamera(opened.camera)
            const video = videoRef.current
            if (video !== null) {
                video.srcObject = opened.stream
                video.play().catch(() => undefined)
            }
            refreshCameras(session)
        },
        [refreshCameras]
    )

    const show = useCallback(
        (session: CameraSession, opened: OpenedCamera) =>
            keepOpen(session, opened, display, () => {
                if (sessionRef.current === session) {
                    dispatch({type: "problem", problem: CaptureProblem.Camera})
                }
            }),
        [display, dispatch]
    )

    useEffect(() => {
        const devices = navigator.mediaDevices
        if (devices?.getUserMedia === undefined) {
            dispatch({type: "problem", problem: CaptureProblem.Camera})
            return
        }
        const session = new CameraSession(devices, HOLDING_CAMERA)
        sessionRef.current = session
        // A webcam plugged in or out changes the cameras to switch between.
        const changed = () => refreshCameras(session)
        devices.addEventListener?.("devicechange", changed)
        void session.start().then((opened) => {
            if (sessionRef.current !== session) return
            if (opened === null) {
                dispatch({type: "problem", problem: CaptureProblem.Camera})
            } else {
                show(session, opened)
            }
        })
        return () => {
            devices.removeEventListener?.("devicechange", changed)
            sessionRef.current = null
            session.close()
        }
    }, [dispatch, refreshCameras, show])

    const switchCamera = async () => {
        const session = sessionRef.current
        if (session === null || switching) return
        setSwitching(true)
        setSwitchFailed(false)
        // The camera being left stops at once, and its last frame stays on
        // screen, perfectly still: no photo is taken until the next one shows.
        setReady(false)
        setProgress(0)
        const result = await session.switch(cameras)
        if (sessionRef.current !== session) return
        setSwitching(false)
        switch (result.outcome) {
            case SwitchOutcome.Switched:
                if (result.opened !== null) show(session, result.opened)
                return
            case SwitchOutcome.Kept:
                if (result.opened === null) {
                    // Nothing to switch to: the same camera never stopped.
                    setReady(true)
                } else {
                    // The camera the voter had, open again: they can carry on with it.
                    show(session, result.opened)
                    setSwitchFailed(true)
                }
                return
            case SwitchOutcome.Lost:
                dispatch({type: "problem", problem: CaptureProblem.Camera})
                return
        }
    }

    // Samples the image while there is no photo, and takes it once still.
    useEffect(() => {
        const video = videoRef.current
        if (!ready || photo !== null || video === null) return
        const sampler = document.createElement("canvas")
        sampler.width = SAMPLE_WIDTH
        sampler.height = SAMPLE_HEIGHT
        const context = sampler.getContext("2d", {willReadFrequently: true})
        if (context === null) {
            dispatch({type: "problem", problem: CaptureProblem.Camera})
            return
        }
        let stillness: Stillness = STILL
        let taking = false
        const timer = setInterval(() => {
            if (
                taking ||
                video.readyState < HTMLMediaElement.HAVE_CURRENT_DATA ||
                !showsLiveCamera(video)
            ) {
                return
            }
            context.drawImage(video, 0, 0, SAMPLE_WIDTH, SAMPLE_HEIGHT)
            const sample = grayscale(context.getImageData(0, 0, SAMPLE_WIDTH, SAMPLE_HEIGHT).data)
            const step = trackStillness(
                stillness,
                sample,
                performance.now(),
                holding.seconds * 1000
            )
            stillness = step.state
            setProgress(step.progress)
            if (!step.captured) return
            taking = true
            clearInterval(timer)
            void takePhoto(video).then((blob) => {
                if (blob === null) {
                    dispatch({type: "problem", problem: CaptureProblem.Camera})
                    return
                }
                setPhoto({blob, url: URL.createObjectURL(blob)})
            })
        }, SAMPLE_INTERVAL_MS)
        return () => clearInterval(timer)
    }, [ready, photo, holding.seconds, dispatch])

    useEffect(() => () => (photo === null ? undefined : URL.revokeObjectURL(photo.url)), [photo])

    const use = async () => {
        if (photo === null) return
        setSending(true)
        const problem = await uploadHoldingPhoto(holding, photo.blob)
        setSending(false)
        dispatch(problem === null ? {type: "done"} : {type: "problem", problem})
    }

    const retake = () => {
        setSwitchFailed(false)
        setProgress(0)
        setPhoto(null)
    }

    const cameraLabel = text("scanovateHoldingCameraLabel")
    const photoLabel = text("scanovateHoldingPhotoLabel")
    const progressLabel = text("scanovateHoldingProgress")
    const status =
        progress > 0
            ? text("scanovateHoldingCountdown", String(secondsLeft(progress, holding.seconds)))
            : text("scanovateHoldingKeepStill")
    const retakeLabel = text("scanovateHoldingRetake")
    const useLabel = text("scanovateHoldingUse")
    const closeLabel = text("scanovateHoldingClose")
    const switchLabel = text("scanovateHoldingSwitchCamera")
    const switchFailedText = text("scanovateHoldingSwitchFailed")
    const switchShown = photo === null && canSwitch(HOLDING_CAMERA, cameras)
    return (
        <>
            <div className="scanovate-holding">
                {/* Kept playing under the preview, so that taking it again is instant. */}
                <video
                    ref={videoRef}
                    aria-label={cameraLabel.text}
                    lang={cameraLabel.lang}
                    hidden={photo !== null}
                    className={mirrorsPreview(camera) ? "mirrored" : undefined}
                    autoPlay
                    muted
                    playsInline
                    onLoadedData={(event) => {
                        if (showsLiveCamera(event.currentTarget)) setReady(true)
                    }}
                />
                {photo !== null && (
                    <img src={photo.url} alt={photoLabel.text} lang={photoLabel.lang} />
                )}
            </div>
            <button
                type="button"
                className="scanovate-holding-close"
                aria-label={closeLabel.text}
                lang={closeLabel.lang}
                disabled={sending}
                onClick={() => dispatch({type: "problem", problem: CaptureProblem.Closed})}
            >
                <CloseIcon />
            </button>
            {switchShown && (
                <button
                    type="button"
                    className="scanovate-holding-switch"
                    aria-label={switchLabel.text}
                    lang={switchLabel.lang}
                    aria-busy={switching}
                    disabled={switching || sending}
                    onClick={() => void switchCamera()}
                >
                    <SwitchCameraIcon />
                </button>
            )}
            {photo === null ? (
                <div className="scanovate-holding-status">
                    {switchFailed && (
                        <p role="alert" lang={switchFailedText.lang}>
                            {switchFailedText.text}
                        </p>
                    )}
                    <p aria-live="polite" lang={status.lang}>
                        {status.text}
                    </p>
                    <div
                        className="scanovate-holding-progress"
                        role="progressbar"
                        aria-label={progressLabel.text}
                        lang={progressLabel.lang}
                        aria-valuemin={0}
                        aria-valuemax={100}
                        aria-valuenow={Math.round(progress * 100)}
                    >
                        <span style={{width: `${progress * 100}%`}} />
                    </div>
                </div>
            ) : (
                <Box className="auth-actions scanovate-holding-actions">
                    <Button
                        type="button"
                        variant="contained"
                        fullWidth
                        className="auth-submit"
                        endIcon={<ArrowIcon />}
                        disabled={sending}
                        onClick={use}
                        lang={useLabel.lang}
                    >
                        {useLabel.text}
                    </Button>
                    <Button
                        type="button"
                        variant="outlined"
                        fullWidth
                        disabled={sending}
                        onClick={retake}
                        lang={retakeLabel.lang}
                    >
                        {retakeLabel.text}
                    </Button>
                </Box>
            )}
        </>
    )
}
