// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, useRef, useState, type Dispatch} from "react"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import type {ScanovateUpload} from "../KcContext"
import {ArrowIcon} from "../icons"
import type {CaptureEvent} from "./capture"
import {takePhoto, uploadHoldingPhoto} from "./holding"
import type {Text} from "./text"
import {CaptureProblem} from "./types"

type Photo = {blob: Blob; url: string}

function stop(stream: MediaStream) {
    for (const track of stream.getTracks()) track.stop()
}

/**
 * The page's own camera: the voter takes a photo of themselves holding their
 * document, checks it, and the page uploads it to Keycloak before moving on.
 */
export function HoldingCamera(props: {
    text: Text
    upload: ScanovateUpload
    dispatch: Dispatch<CaptureEvent>
}) {
    const {text, upload, dispatch} = props
    const videoRef = useRef<HTMLVideoElement | null>(null)
    const [ready, setReady] = useState(false)
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

    useEffect(() => () => (photo === null ? undefined : URL.revokeObjectURL(photo.url)), [photo])

    const take = async () => {
        const video = videoRef.current
        const blob = video === null ? null : await takePhoto(video)
        if (blob === null) {
            dispatch({type: "problem", problem: CaptureProblem.Camera})
            return
        }
        setPhoto({blob, url: URL.createObjectURL(blob)})
    }

    const use = async () => {
        if (photo === null) return
        setSending(true)
        const problem = await uploadHoldingPhoto(upload, photo.blob)
        setSending(false)
        dispatch(problem === null ? {type: "done"} : {type: "problem", problem})
    }

    const cameraLabel = text("scanovateHoldingCameraLabel")
    const photoLabel = text("scanovateHoldingPhotoLabel")
    const takeLabel = text("scanovateHoldingTake")
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
            <Box className="auth-actions">
                {photo === null ? (
                    <Button
                        type="button"
                        variant="contained"
                        fullWidth
                        className="auth-submit"
                        endIcon={<ArrowIcon />}
                        disabled={!ready}
                        onClick={take}
                        lang={takeLabel.lang}
                    >
                        {takeLabel.text}
                    </Button>
                ) : (
                    <>
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
                            onClick={() => setPhoto(null)}
                            lang={retakeLabel.lang}
                        >
                            {retakeLabel.text}
                        </Button>
                    </>
                )}
            </Box>
        </>
    )
}
