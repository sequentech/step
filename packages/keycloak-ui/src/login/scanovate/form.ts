// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Captures} from "./controller"
import {CaptureStep} from "./types"

export const CAPTURE_ACTION = "capture"

// Parts of the capture, as the scanovate-authenticator names them.
export enum CapturePart {
    Front = "front",
    Back = "back",
    Holding = "holding",
}

export type CaptureUpload = {part: CapturePart; blob: Blob}

// Keycloak takes the voter's face from Liveness Plus server to server, so only
// the ID and the photo of the voter holding it are uploaded.
const PARTS: [CaptureStep, CapturePart][] = [
    [CaptureStep.Front, CapturePart.Front],
    [CaptureStep.Back, CapturePart.Back],
    [CaptureStep.Video, CapturePart.Holding],
]

/** The captures to upload, in order; steps not captured, such as the back of a passport, are left out. */
export function captureParts(captures: Captures): CaptureUpload[] {
    return PARTS.flatMap(([step, part]) => {
        const blob = captures[step]
        return blob === undefined ? [] : [{part, blob}]
    })
}
