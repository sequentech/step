// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {MessageKey} from "../i18n"
import {CaptureStep, DocumentStatus, FaceStatus} from "./types"

export enum Tone {
    Guide = "GUIDE",
    Ok = "OK",
}

export type Guidance = {message: MessageKey; tone: Tone}

const LOADING: Guidance = {message: "scanovateGuideLoading", tone: Tone.Guide}

const DOCUMENT: Record<DocumentStatus, MessageKey> = {
    [DocumentStatus.NoDocument]: "scanovateGuidePlaceDocument",
    [DocumentStatus.TooFar]: "scanovateGuideDocumentTooFar",
    [DocumentStatus.TooClose]: "scanovateGuideDocumentTooClose",
    [DocumentStatus.NotAligned]: "scanovateGuideDocumentNotAligned",
    [DocumentStatus.TooDark]: "scanovateGuideTooDark",
    [DocumentStatus.TooBright]: "scanovateGuideDocumentTooBright",
    [DocumentStatus.Glare]: "scanovateGuideGlare",
    [DocumentStatus.Blurry]: "scanovateGuideDocumentBlurry",
    [DocumentStatus.HoldStill]: "scanovateGuideDocumentHoldStill",
    [DocumentStatus.Ready]: "scanovateGuideDocumentHoldStill",
}

const FACE: Record<FaceStatus, MessageKey> = {
    [FaceStatus.NoFace]: "scanovateGuidePlaceFace",
    [FaceStatus.MultipleFaces]: "scanovateGuideMultipleFaces",
    [FaceStatus.TooFar]: "scanovateGuideFaceTooFar",
    [FaceStatus.TooClose]: "scanovateGuideFaceTooClose",
    [FaceStatus.OffCenter]: "scanovateGuideFaceOffCenter",
    [FaceStatus.TurnToCamera]: "scanovateGuideTurnToCamera",
    [FaceStatus.TooDark]: "scanovateGuideTooDark",
    [FaceStatus.TooBright]: "scanovateGuideFaceTooBright",
    [FaceStatus.Blurry]: "scanovateGuideFaceBlurry",
    [FaceStatus.HoldStill]: "scanovateGuideFaceHoldStill",
    [FaceStatus.Ready]: "scanovateGuideFaceHoldStill",
}

export function documentGood(status: DocumentStatus | null): boolean {
    return status === DocumentStatus.HoldStill || status === DocumentStatus.Ready
}

export function faceGood(status: FaceStatus | null): boolean {
    return status === FaceStatus.HoldStill || status === FaceStatus.Ready
}

// Held next to the face the ID only has to be in the small frame, not fill it.
export function documentVisible(status: DocumentStatus | null): boolean {
    return status !== null && status !== DocumentStatus.NoDocument
}

export function documentGuidance(step: CaptureStep, status: DocumentStatus | null): Guidance {
    if (status === null) return LOADING
    if (step === CaptureStep.Back && status === DocumentStatus.NoDocument) {
        return {message: "scanovateGuideTurnDocument", tone: Tone.Guide}
    }
    return {message: DOCUMENT[status], tone: documentGood(status) ? Tone.Ok : Tone.Guide}
}

export function faceGuidance(status: FaceStatus | null): Guidance {
    if (status === null) return LOADING
    return {message: FACE[status], tone: faceGood(status) ? Tone.Ok : Tone.Guide}
}

export function videoGuidance(
    face: FaceStatus | null,
    document: DocumentStatus | null,
    recording: boolean
): Guidance {
    if (face === null) return LOADING
    const visible = documentVisible(document)
    if (recording) {
        return faceGood(face) && visible
            ? {message: "scanovateGuideRecording", tone: Tone.Ok}
            : {message: "scanovateGuideKeepInView", tone: Tone.Guide}
    }
    if (!faceGood(face)) return faceGuidance(face)
    if (!visible) return {message: "scanovateGuideHoldDocument", tone: Tone.Guide}
    return {message: "scanovateGuideFaceHoldStill", tone: Tone.Ok}
}

// The shutter is a fallback: it only works once the frame is good.
export function canTakePhoto(status: DocumentStatus | FaceStatus | null): boolean {
    return (
        status === DocumentStatus.Ready ||
        status === DocumentStatus.HoldStill ||
        status === FaceStatus.Ready ||
        status === FaceStatus.HoldStill
    )
}
