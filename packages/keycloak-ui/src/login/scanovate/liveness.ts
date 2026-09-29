// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// The Scanovate Liveness Plus iframe that checks the voter's liveness. It only
// tells us when it is done or failed, so that the page can move on: the verdict
// reaches Keycloak server to server and is never taken from these messages.
import {CaptureProblem} from "./types"

export enum LivenessEventType {
    Init = "init",
    Done = "done",
    Error = "error",
}

export type LivenessEvent = {type: LivenessEventType; errorCode: number | null}

// Error codes of Liveness Plus.
const CAMERA_DENIED = [1003, 1013]
const CAMERA_NOT_FOUND = [1004]
const CAMERA_FAILED = [1005, 1016]
const CLOSED = [1009, 1010, 1015]
const INVALID_TOKEN = 1014

const EVENT_TYPES = new Set<string>(Object.values(LivenessEventType))

export function parseLivenessEvent(data: unknown): LivenessEvent | null {
    let message = data
    if (typeof message === "string") {
        try {
            message = JSON.parse(message)
        } catch {
            return null
        }
    }
    if (typeof message !== "object" || message === null) return null
    const {type, error_code: errorCode} = message as {type?: unknown; error_code?: unknown}
    if (typeof type !== "string" || !EVENT_TYPES.has(type)) return null
    const code = Number(errorCode)
    return {
        type: type as LivenessEventType,
        errorCode: errorCode === undefined || Number.isNaN(code) ? null : code,
    }
}

// The problem to show for a Liveness Plus error, or null when the voter closed it.
export function livenessProblem(errorCode: number | null): CaptureProblem | null {
    if (errorCode === null) return CaptureProblem.LivenessFailed
    if (CLOSED.includes(errorCode)) return null
    if (CAMERA_DENIED.includes(errorCode)) return CaptureProblem.CameraDenied
    if (CAMERA_NOT_FOUND.includes(errorCode)) return CaptureProblem.CameraNotFound
    if (CAMERA_FAILED.includes(errorCode)) return CaptureProblem.CameraFailed
    if (errorCode === INVALID_TOKEN) return CaptureProblem.LivenessExpired
    return CaptureProblem.LivenessFailed
}

// Adds the voter's language, if Liveness Plus has it, to the URL from Keycloak.
export function livenessFrameUrl(url: string, languages: string[], languageTag: string): string {
    const language = languageTag.split("-")[0]
    if (!languages.includes(language)) return url
    return `${url}${url.includes("?") ? "&" : "?"}translation_language=${encodeURIComponent(language)}`
}
