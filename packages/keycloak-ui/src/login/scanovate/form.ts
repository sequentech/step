// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Captures} from "./controller"
import {CaptureStep, FaceCheck} from "./types"

export const CAPTURE_ACTION = "capture"

// Parts of the capture, as the scanovate-authenticator names them.
export enum CapturePart {
    Front = "front",
    Back = "back",
    Face = "face",
    Video = "video",
    Holding = "holding",
}

export type CaptureUpload = {part: CapturePart; blob: Blob}

// With liveness, Keycloak takes the voter's face from Liveness Plus server to
// server, and the step holding the ID is a photo instead of a video.
const PARTS: Record<FaceCheck, [CaptureStep, CapturePart][]> = {
    [FaceCheck.Photo]: [
        [CaptureStep.Front, CapturePart.Front],
        [CaptureStep.Back, CapturePart.Back],
        [CaptureStep.Face, CapturePart.Face],
        [CaptureStep.Video, CapturePart.Video],
    ],
    [FaceCheck.Liveness]: [
        [CaptureStep.Front, CapturePart.Front],
        [CaptureStep.Back, CapturePart.Back],
        [CaptureStep.Video, CapturePart.Holding],
    ],
}

/** The captures to upload, in order; steps not captured, such as the back of a passport, are left out. */
export function captureParts(captures: Captures, check: FaceCheck): CaptureUpload[] {
    return PARTS[check].flatMap(([step, part]) => {
        const blob = captures[step]
        return blob === undefined ? [] : [{part, blob}]
    })
}
