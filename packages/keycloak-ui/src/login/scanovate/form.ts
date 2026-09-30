// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Captures} from "./controller"
import {CaptureStep, FaceCheck} from "./types"

export const CAPTURE_ACTION = "capture"

// Multipart part names expected by the scanovate-authenticator.
export enum CapturePart {
    Front = "front",
    Back = "back",
    Face = "face",
    Video = "video",
    Holding = "holding",
}

// With liveness, Keycloak takes the voter's face from Liveness Plus server to
// server, and the step holding the ID is a photo instead of a video.
const PARTS: Record<FaceCheck, Partial<Record<CaptureStep, CapturePart>>> = {
    [FaceCheck.Photo]: {
        [CaptureStep.Front]: CapturePart.Front,
        [CaptureStep.Back]: CapturePart.Back,
        [CaptureStep.Face]: CapturePart.Face,
        [CaptureStep.Video]: CapturePart.Video,
    },
    [FaceCheck.Liveness]: {
        [CaptureStep.Front]: CapturePart.Front,
        [CaptureStep.Back]: CapturePart.Back,
        [CaptureStep.Video]: CapturePart.Holding,
    },
}

const EXTENSIONS: Record<string, string> = {
    "image/jpeg": "jpg",
    "video/webm": "webm",
    "video/mp4": "mp4",
}

export function captureFile(part: CapturePart, blob: Blob): File {
    const type = blob.type.split(";")[0]
    const extension = EXTENSIONS[type] ?? "bin"
    return new File([blob], `${part}.${extension}`, {type})
}

/** Puts every capture in its hidden file input; inputs without a capture are emptied. */
export function populateCaptureForm(
    form: HTMLFormElement,
    captures: Captures,
    check: FaceCheck
): void {
    const parts = PARTS[check]
    const blobs = new Map<CapturePart, Blob>()
    for (const [step, part] of Object.entries(parts) as [CaptureStep, CapturePart][]) {
        const blob = captures[step]
        if (blob !== undefined) blobs.set(part, blob)
    }
    for (const part of Object.values(CapturePart)) {
        const input = form.elements.namedItem(part)
        if (!(input instanceof HTMLInputElement)) continue
        const transfer = new DataTransfer()
        const blob = blobs.get(part)
        if (blob !== undefined) {
            transfer.items.add(captureFile(part, blob))
        }
        input.files = transfer.files
        input.disabled = blob === undefined
    }
}
