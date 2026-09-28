// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Captures} from "./controller"
import {CaptureStep} from "./types"

export const CAPTURE_ACTION = "capture"

// Multipart part names expected by the scanovate-authenticator.
export const CAPTURE_PARTS: Record<CaptureStep, string> = {
    [CaptureStep.Front]: "front",
    [CaptureStep.Back]: "back",
    [CaptureStep.Face]: "face",
    [CaptureStep.Video]: "video",
}

const EXTENSIONS: Record<string, string> = {
    "image/jpeg": "jpg",
    "video/webm": "webm",
    "video/mp4": "mp4",
}

export function captureFile(step: CaptureStep, blob: Blob): File {
    const type = blob.type.split(";")[0]
    const extension = EXTENSIONS[type] ?? "bin"
    return new File([blob], `${CAPTURE_PARTS[step]}.${extension}`, {type})
}

/** Puts every capture in its hidden file input; inputs without a capture are emptied. */
export function populateCaptureForm(form: HTMLFormElement, captures: Captures): void {
    for (const step of Object.values(CaptureStep)) {
        const input = form.elements.namedItem(CAPTURE_PARTS[step])
        if (!(input instanceof HTMLInputElement)) continue
        const transfer = new DataTransfer()
        const blob = captures[step]
        if (blob !== undefined) {
            transfer.items.add(captureFile(step, blob))
        }
        input.files = transfer.files
        input.disabled = blob === undefined
    }
}
