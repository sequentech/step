// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {
    CaptureProblem,
    CaptureStage,
    ScanovateErrorCode,
    ScanovateMessageKind,
    type ScanovateMessage,
} from "./types"

// The verdict never comes from the browser: Scanovate posts its results to
// Keycloak server to server. The page only follows the steps.

/** Reads what a Scanovate page posted to its parent; null for what the page ignores. */
export function parseScanovateMessage(data: unknown): ScanovateMessage | null {
    if (typeof data !== "object" || data === null || !("type" in data)) {
        return null
    }
    switch (data.type) {
        case "done":
            return {kind: ScanovateMessageKind.Done}
        case "error":
            return {
                kind: ScanovateMessageKind.Error,
                code: "error_code" in data ? errorCode(data.error_code) : null,
            }
        default:
            return null
    }
}

function errorCode(value: unknown): number | null {
    const code = typeof value === "string" && value.trim() !== "" ? Number(value) : value
    return typeof code === "number" && Number.isInteger(code) ? code : null
}

const PROBLEMS: Partial<Record<number, CaptureProblem>> = {
    [ScanovateErrorCode.CameraPermissionBlocked]: CaptureProblem.Camera,
    [ScanovateErrorCode.CameraNotFound]: CaptureProblem.Camera,
    [ScanovateErrorCode.CouldNotOpenCamera]: CaptureProblem.Camera,
    [ScanovateErrorCode.CameraPermissionDismissed]: CaptureProblem.Camera,
    [ScanovateErrorCode.SessionExpired]: CaptureProblem.Expired,
    [ScanovateErrorCode.InvalidToken]: CaptureProblem.InvalidToken,
    [ScanovateErrorCode.UserPressedClose]: CaptureProblem.Closed,
}

export function problemFor(code: number | null): CaptureProblem {
    return (code === null ? undefined : PROBLEMS[code]) ?? CaptureProblem.Generic
}

export type CaptureState = {
    stage: CaptureStage
    problem: CaptureProblem | null
    // Bumped to mount the step again: a Scanovate token allows a few sessions.
    attempt: number
    // Whether Keycloak asks for the photo of the voter holding their document.
    holding: boolean
}

export type CaptureEvent =
    | {type: "done"}
    | {type: "error"; code: number | null}
    | {type: "problem"; problem: CaptureProblem}
    | {type: "tryAgain"}

export function initialCaptureState(holding = false): CaptureState {
    return {stage: CaptureStage.Document, problem: null, attempt: 0, holding}
}

function next(state: CaptureState): CaptureStage {
    switch (state.stage) {
        case CaptureStage.Document:
            return CaptureStage.Face
        case CaptureStage.Face:
            return state.holding ? CaptureStage.Holding : CaptureStage.Checking
        case CaptureStage.Holding:
        case CaptureStage.Checking:
            return CaptureStage.Checking
    }
}

export function captureReducer(state: CaptureState, event: CaptureEvent): CaptureState {
    switch (event.type) {
        case "done":
            if (state.problem !== null || state.stage === CaptureStage.Checking) return state
            return {...state, stage: next(state)}
        case "error":
            if (state.problem !== null || state.stage === CaptureStage.Checking) return state
            return {...state, problem: problemFor(event.code)}
        case "problem":
            if (state.problem !== null || state.stage === CaptureStage.Checking) return state
            return {...state, problem: event.problem}
        case "tryAgain":
            if (state.problem === null) return state
            return {...state, problem: null, attempt: state.attempt + 1}
    }
}

export function captureEvent(message: ScanovateMessage): CaptureEvent {
    return message.kind === ScanovateMessageKind.Done
        ? {type: "done"}
        : {type: "error", code: message.code}
}

/**
 * Only the embedded page that is showing speaks for the capture: its window
 * posted the message, from the page's own origin or the one of its URL.
 */
export function acceptsMessage(
    event: {source: unknown; origin: string},
    frame: Window | null | undefined,
    pageOrigin: string,
    frameUrl: string
): boolean {
    if (frame === null || frame === undefined || event.source !== frame) {
        return false
    }
    if (event.origin === pageOrigin) {
        return true
    }
    try {
        return event.origin === new URL(frameUrl, pageOrigin).origin
    } catch {
        return false
    }
}
