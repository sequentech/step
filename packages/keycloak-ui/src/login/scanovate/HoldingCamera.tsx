// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, useRef, useState, type Dispatch} from "react"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import type {ScanovateHolding} from "../KcContext"
import {ArrowIcon} from "../icons"
import type {CaptureEvent} from "./capture"
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

// How often the camera image is sampled for movement.
const SAMPLE_INTERVAL_MS = 100

function stop(stream: MediaStream) {
    for (const track of stream.getTracks()) track.stop()
}

/**
 * The page's own camera: the voter holds their document next to their face,
 * the page takes the photo by itself once the image has been still for the
 * holding time, the voter checks it, and the page uploads it to Keycloak
 * before moving on. There is no shutter button: the photo is only ever taken
 * still.
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

    useEffect(() => {
        let stream: MediaStream | null = null
        let stopped = false
        if (navigator.mediaDevices?.getUserMedia === undefined) {
            dispatch({type: "problem", problem: CaptureProblem.Camera})
            return
        }
        navigator.mediaDevices
            .getUserMedia({
                video: {facingMode: "user", width: {ideal: 1280}, height: {ideal: 720}},
                audio: false,
            })
            .then((opened) => {
                if (stopped) {
                    stop(opened)
                    return
                }
                stream = opened
                const video = videoRef.current
                if (video !== null) {
                    video.srcObject = opened
                    video.play().catch(() => undefined)
                }
            })
            .catch(() => dispatch({type: "problem", problem: CaptureProblem.Camera}))
        return () => {
            stopped = true
            if (stream !== null) stop(stream)
        }
    }, [dispatch])

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
            if (taking || video.readyState < HTMLMediaElement.HAVE_CURRENT_DATA) return
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
    return (
        <>
            <div className="scanovate-holding">
                {/* Kept playing under the preview, so that taking it again is instant. */}
                <video
                    ref={videoRef}
                    aria-label={cameraLabel.text}
                    lang={cameraLabel.lang}
                    hidden={photo !== null}
                    autoPlay
                    muted
                    playsInline
                    onLoadedData={() => setReady(true)}
                />
                {photo !== null && (
                    <img src={photo.url} alt={photoLabel.text} lang={photoLabel.lang} />
                )}
            </div>
            {photo === null ? (
                <div className="scanovate-holding-status">
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
                <Box className="auth-actions">
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
