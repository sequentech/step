// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {loadAnalyzers} from "./analyzers"
import {StageLayout} from "./geometry"
import {
    CameraFacing,
    CaptureProblem,
    CaptureStep,
    type CameraService,
    type CaptureServices,
} from "./types"
import {fetchLiveness} from "./livenessApi"
import {fetchUploads} from "./uploads"

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

/**
 * Whether to mirror the preview of a camera, like a mirror for a camera facing the voter. Laptops
 * open their webcam whichever camera is asked for, usually without saying which way it faces.
 * The document is never mirrored: its text would read backwards.
 */
export function previewMirrored(
    step: CaptureStep,
    requested: CameraFacing,
    stream: MediaStream | null,
    layout: StageLayout
): boolean {
    if (step === CaptureStep.Front || step === CaptureStep.Back) return false
    const facing = stream?.getVideoTracks()[0]?.getSettings().facingMode
    if (facing === CameraFacing.User) return true
    if (facing === CameraFacing.Environment) return false
    return layout === StageLayout.Desktop || requested === CameraFacing.User
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

export const browserServices: CaptureServices = {
    camera: browserCamera,
    liveness: fetchLiveness,
    uploads: fetchUploads,
    loadAnalyzers,
    vibrate: (pattern) => {
        if (typeof navigator.vibrate === "function") navigator.vibrate(pattern)
    },
}
