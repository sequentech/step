// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {loadAnalyzers} from "./analyzers"
import {
    CameraFacing,
    CaptureProblem,
    type CameraService,
    type CaptureServices,
    type RecorderService,
    type Recording,
} from "./types"

export const VIDEO_BITS_PER_SECOND = 1_500_000
// The authenticator accepts videos up to 3 MiB; keep a margin for the container.
export const VIDEO_MAX_BYTES = 3 * 1024 * 1024
const VIDEO_BUDGET = 0.8

export function videoBitsPerSecond(seconds: number): number {
    const budget = Math.floor((VIDEO_MAX_BYTES * 8 * VIDEO_BUDGET) / Math.max(seconds, 1))
    return Math.min(VIDEO_BITS_PER_SECOND, budget)
}
// WebM for Chromium and Firefox, MP4 for Safari.
export const VIDEO_MIME_TYPES = [
    "video/webm;codecs=vp8",
    "video/webm;codecs=vp9",
    "video/webm",
    "video/mp4;codecs=avc1",
    "video/mp4",
]

export class InsecureContextError extends Error {
    constructor() {
        super("The camera is only available in a secure context")
        this.name = "InsecureContextError"
    }
}

export function cameraProblem(error: unknown): CaptureProblem {
    const name = error instanceof Error ? error.name : ""
    switch (name) {
        case "InsecureContextError":
            return CaptureProblem.InsecureContext
        case "NotAllowedError":
        case "SecurityError":
        case "PermissionDeniedError":
            return CaptureProblem.CameraDenied
        case "NotFoundError":
        case "OverconstrainedError":
        case "DevicesNotFoundError":
            return CaptureProblem.CameraNotFound
        case "NotReadableError":
        case "TrackStartError":
        case "AbortError":
            return CaptureProblem.CameraInUse
        default:
            return CaptureProblem.CameraFailed
    }
}

export function stopStream(stream: MediaStream): void {
    for (const track of stream.getTracks()) {
        track.stop()
    }
}

export const browserCamera: CameraService = {
    async open(facing: CameraFacing): Promise<MediaStream> {
        if (!window.isSecureContext || navigator.mediaDevices?.getUserMedia === undefined) {
            throw new InsecureContextError()
        }
        return navigator.mediaDevices.getUserMedia({
            audio: false,
            video: {
                facingMode: {ideal: facing},
                width: {ideal: 1920},
                height: {ideal: 1080},
            },
        })
    },
}

export function pickMimeType(isTypeSupported: (type: string) => boolean): string | null {
    return VIDEO_MIME_TYPES.find((type) => isTypeSupported(type)) ?? null
}

function recorderMimeType(): string | null {
    if (typeof MediaRecorder === "undefined") return null
    return pickMimeType((type) => MediaRecorder.isTypeSupported(type))
}

export const browserRecorder: RecorderService = {
    supported: () => recorderMimeType() !== null,
    start(stream: MediaStream, seconds: number): Recording {
        const mimeType = recorderMimeType()
        if (mimeType === null) {
            throw new Error("MediaRecorder cannot record video in this browser")
        }
        const recorder = new MediaRecorder(stream, {
            mimeType,
            videoBitsPerSecond: videoBitsPerSecond(seconds),
        })
        const chunks: Blob[] = []
        recorder.addEventListener("dataavailable", (event) => {
            if (event.data.size > 0) chunks.push(event.data)
        })
        recorder.start(250)
        return {
            stop: () =>
                new Promise<Blob>((resolve, reject) => {
                    recorder.addEventListener(
                        "stop",
                        () => resolve(new Blob(chunks, {type: mimeType.split(";")[0]})),
                        {once: true}
                    )
                    recorder.addEventListener(
                        "error",
                        () => reject(new Error("Recording failed")),
                        {
                            once: true,
                        }
                    )
                    recorder.stop()
                }),
            cancel: () => {
                if (recorder.state !== "inactive") recorder.stop()
                chunks.length = 0
            },
        }
    },
}

export const browserServices: CaptureServices = {
    camera: browserCamera,
    recorder: browserRecorder,
    loadAnalyzers,
    vibrate: (pattern) => {
        if (typeof navigator.vibrate === "function") navigator.vibrate(pattern)
    },
}
