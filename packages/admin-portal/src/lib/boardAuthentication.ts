// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {KeycloakLoginOptions} from "keycloak-js"
import {SigningAction, type ISigningRequest} from "./signing/types"

/** Mirror the server boundary; refreshed token issuance is not authentication. */
export const hasFreshBoardAuthentication = (
    token: {auth_time?: unknown} | undefined,
    now = Date.now()
): boolean => {
    const at = token?.auth_time
    if (typeof at !== "number" || !Number.isSafeInteger(at) || at < 0) return false
    const age = now - at * 1000
    return age >= 0 && age < 60_000
}

export const boardLoginOptions = (redirectUri: string): KeycloakLoginOptions => ({
    redirectUri,
    prompt: "login",
    maxAge: 60,
})

export const requiresFreshBoardAuthentication = (action: SigningAction): boolean =>
    action !== SigningAction.ConfirmCustodyProof && action !== SigningAction.ContributeCustodyProof

export interface BoardActionScope {
    tenantId: string
    userId: string
    eventId: string
    electionId: string | null
}
const MAX_RESUME_AGE = 5 * 60_000
interface PendingBoardAction extends BoardActionScope {
    action: string
    createdAt: number
    channels?: string[]
}
export const savePendingBoardAction = (
    storage: Storage,
    key: string,
    scope: BoardActionScope,
    channels?: string[],
    now = Date.now()
): void => {
    if (!scope.tenantId || !scope.userId || !scope.eventId) throw new Error("Missing action scope")
    storage.setItem(key, JSON.stringify({...scope, action: key, createdAt: now, channels}))
}

export type PendingBoardResult =
    | {status: "none" | "waiting" | "refused"}
    | {status: "ready"; channels?: string[]}

/** A failed fresh login keeps the exact intent; it never initiates another login automatically. */
export const consumePendingBoardAction = (
    storage: Storage,
    key: string,
    scope: BoardActionScope,
    fresh: boolean,
    now = Date.now()
): PendingBoardResult => {
    const raw = storage.getItem(key)
    if (!raw) return {status: "none"}
    try {
        const value = JSON.parse(raw) as PendingBoardAction
        if (
            !scope.tenantId ||
            !scope.userId ||
            !scope.eventId ||
            value.action !== key ||
            value.tenantId !== scope.tenantId ||
            value.userId !== scope.userId ||
            value.eventId !== scope.eventId ||
            value.electionId !== scope.electionId ||
            !Number.isSafeInteger(value.createdAt) ||
            now < value.createdAt ||
            now - value.createdAt >= MAX_RESUME_AGE ||
            (value.channels !== undefined &&
                (!Array.isArray(value.channels) ||
                    !value.channels.length ||
                    value.channels.some(
                        (x) => !["ONLINE", "KIOSK", "EARLY_VOTING", "TELEPHONE"].includes(x)
                    )))
        )
            throw new Error("Invalid pending action")
        if (!fresh) return {status: "waiting"}
        storage.removeItem(key)
        return {status: "ready", channels: value.channels}
    } catch {
        storage.removeItem(key)
        return {status: "refused"}
    }
}

export const BOARD_SIGNING_RESUME_KEY = "boardSigningFreshAuthentication"
export interface BoardSigningResume {
    tenantId: string
    userId: string
    eventId: string
    requestId: string
    action: SigningAction
    payloadHash: string
    code: string
    expiresAt: number
}
export const saveBoardSigningResume = (
    storage: Storage,
    request: ISigningRequest,
    userId: string,
    now = Date.now()
): void => {
    if (!userId || !request.tenant_id || !request.election_event_id || !request.id)
        throw new Error("Missing signing scope")
    const expiry = request.expires_at ? Date.parse(request.expires_at) : now + MAX_RESUME_AGE
    const value: BoardSigningResume = {
        tenantId: request.tenant_id,
        userId,
        eventId: request.election_event_id,
        requestId: request.id,
        action: request.action,
        payloadHash: request.payload_sha256,
        code: request.code,
        expiresAt: Math.min(expiry, now + MAX_RESUME_AGE),
    }
    if (!Number.isFinite(value.expiresAt) || value.expiresAt <= now)
        throw new Error("Expired signing request")
    storage.setItem(BOARD_SIGNING_RESUME_KEY, JSON.stringify(value))
}
export const readBoardSigningResume = (
    storage: Storage,
    tenantId: string,
    userId: string,
    now = Date.now()
): BoardSigningResume | null => {
    const raw = storage.getItem(BOARD_SIGNING_RESUME_KEY)
    if (!raw) return null
    try {
        const x = JSON.parse(raw) as BoardSigningResume
        if (
            x.tenantId !== tenantId ||
            x.userId !== userId ||
            !x.eventId ||
            !x.requestId ||
            !Object.values(SigningAction).includes(x.action) ||
            !requiresFreshBoardAuthentication(x.action) ||
            typeof x.payloadHash !== "string" ||
            !/^[a-f0-9]{64}$/.test(x.payloadHash) ||
            typeof x.code !== "string" ||
            !x.code ||
            !Number.isFinite(x.expiresAt) ||
            x.expiresAt <= now ||
            x.expiresAt > now + MAX_RESUME_AGE
        )
            throw new Error("Invalid signing resume")
        return x
    } catch {
        storage.removeItem(BOARD_SIGNING_RESUME_KEY)
        throw new Error("Signing authentication return does not match this person or request")
    }
}
export const matchesBoardSigningResume = (
    x: BoardSigningResume,
    request: ISigningRequest
): boolean =>
    x.tenantId === request.tenant_id &&
    x.eventId === request.election_event_id &&
    x.requestId === request.id &&
    x.action === request.action &&
    x.payloadHash === request.payload_sha256 &&
    x.code === request.code
