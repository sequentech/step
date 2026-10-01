// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// The API of the on-premise Scanovate Liveness Plus service, which our own page
// drives in its presentation check mode: it gets our face frames as plain JPEGs.
// Liveness Plus posts the result to Keycloak server to server; nothing it
// answers here is the verdict.
import type {ScanovateLiveness} from "../KcContext"

// Status codes of Liveness Plus.
export enum LivenessStatus {
    Ok = 0,
    SessionCompleted = 1,
    ScanCompleted = 2,
    TooManyFaces = 10,
    FaceNotFound = 20,
    FaceTooSmall = 30,
    FaceTooLarge = 40,
    FaceTooCloseToBorder = 41,
    FaceNotCentered = 42,
    YawTooLarge = 50,
    PitchTooLarge = 60,
    RollTooLarge = 70,
    FaceNotInFocus = 80,
    BadlyLit = 90,
    SunglassesDetected = 100,
    MaskDetected = 110,
    CouldNotExtractFaceImage = 200,
    CouldNotCheckPresentationAttack = 201,
    CouldNotCheckInjectionAttack = 202,
    TokenMissing = 1000,
    TokenInvalid = 1001,
    SessionExpired = 1002,
    SessionNotFound = 1003,
    ServerError = 5000,
}

// Why a session is ended from the page, as Liveness Plus names it.
export enum LivenessAbort {
    LeftPage = "user left page",
    PressedClose = "user pressed close",
}

export enum LivenessRejection {
    // Keycloak rejected the one-time token: only a new page gets a new one.
    InvalidToken = "INVALID_TOKEN",
    Failed = "FAILED",
}

export class LivenessApiError extends Error {
    constructor(readonly rejection: LivenessRejection) {
        super(`Liveness Plus request failed: ${rejection}`)
    }
}

export interface LivenessApi {
    // Resolves to the session token.
    createSession(): Promise<string>
    // Resolves to the status of the frame.
    checkFrame(session: string, frame: Blob): Promise<number>
    // Ends a scanned session, which makes Liveness Plus send its result to Keycloak.
    completeSession(session: string): Promise<number>
    // Best effort: never fails.
    abort(session: string, reason: LivenessAbort): Promise<void>
}

export type LivenessConnector = (settings: ScanovateLiveness) => LivenessApi

const SESSION_HEADER = "service-session-token"

async function request(url: string, init: RequestInit): Promise<Response> {
    let response: Response
    try {
        response = await fetch(url, {...init, credentials: "omit"})
    } catch {
        throw new LivenessApiError(LivenessRejection.Failed)
    }
    if (response.status === 401) throw new LivenessApiError(LivenessRejection.InvalidToken)
    if (!response.ok) throw new LivenessApiError(LivenessRejection.Failed)
    return response
}

async function body(response: Response): Promise<Record<string, unknown>> {
    try {
        const parsed: unknown = await response.json()
        if (typeof parsed === "object" && parsed !== null) return parsed as Record<string, unknown>
    } catch {
        // Handled below.
    }
    throw new LivenessApiError(LivenessRejection.Failed)
}

async function statusCode(response: Response): Promise<number> {
    const status = (await body(response)).status as {code?: unknown} | undefined
    if (typeof status?.code !== "number") throw new LivenessApiError(LivenessRejection.Failed)
    return status.code
}

export const fetchLiveness: LivenessConnector = ({url, token, caseId}) => {
    const base = url.replace(/\/+$/, "")
    return {
        async createSession() {
            const response = await request(`${base}/create_session`, {
                method: "POST",
                headers: {"content-type": "application/json"},
                body: JSON.stringify({token, case_id: caseId}),
            })
            const session = (await body(response)).session_token
            if (typeof session !== "string" || session === "") {
                throw new LivenessApiError(LivenessRejection.Failed)
            }
            return session
        },
        async checkFrame(session, frame) {
            const form = new FormData()
            form.append("encrypted_file", frame, "frame.jpg")
            form.append("timestamp", new Date().toISOString())
            form.append("frame_id", crypto.randomUUID())
            const response = await request(`${base}/check_liveness`, {
                method: "POST",
                headers: {[SESSION_HEADER]: session},
                body: form,
            })
            return statusCode(response)
        },
        async completeSession(session) {
            const response = await request(`${base}/client_session_data`, {
                headers: {[SESSION_HEADER]: session},
            })
            return statusCode(response)
        },
        async abort(session, reason) {
            try {
                await fetch(`${base}/client_error`, {
                    method: "POST",
                    headers: {"content-type": "application/json", [SESSION_HEADER]: session},
                    body: JSON.stringify({error: reason}),
                    credentials: "omit",
                    // The request outlives the page when the voter leaves.
                    keepalive: true,
                })
            } catch {
                // The session expires on its own.
            }
        },
    }
}
