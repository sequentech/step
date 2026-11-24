// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {decodeTabMarker, isId, type ITabMarker} from "./operationalGoldReturn"
import {DocumentKind, SIGNING_ACTIONS, SigningAction} from "./types"

const TAB = "sequent.operational-signing.product.v2"
const CLIENT = /^[a-zA-Z0-9_.-]{1,128}$/
const actions = new Set<string>(Object.values(SigningAction))

export interface ProductApprovalScope {
    tenant: string
    event: string
    request: string
    actor: string
    action: SigningAction
}

export type ProductTab = {
    marker: ITabMarker
    action: SigningAction
    turn: "payload" | "artifact"
}

export type ProductReply =
    | {phase: "gold"; attempt: string; authorization_url: string}
    | {phase: "recorded" | "reconcile" | "ready"}

export const record = (v: unknown): Record<string, unknown> =>
    v && typeof v === "object" && !Array.isArray(v) ? (v as Record<string, unknown>) : {}

export const isSigningAction = (value: unknown): value is SigningAction =>
    typeof value === "string" && actions.has(value)

export const productSigningSelected = (mode: unknown, action: unknown, flow: unknown) =>
    mode === "hsm-required" && flow === "product-v2" && isSigningAction(action)

/** A callback with unknown effect cannot be made retryable by changing the rendered view. */
export const productStatusRequired = (
    returned: boolean,
    view: string,
    unresolved: string | undefined,
    ready: string | undefined
) => unresolved === view || (returned && ready !== view)

export const hasArtifact = (action: SigningAction) =>
    SIGNING_ACTIONS[action].document !== DocumentKind.NoDocument

export const scopeOf = ({marker, action}: ProductTab): ProductApprovalScope => ({
    tenant: marker.tenant,
    event: marker.event,
    request: marker.request,
    actor: marker.official,
    action,
})

function decodeProductTab(value: unknown): ProductTab | undefined {
    const {marker, action, turn, until} = record(value)
    const decoded = decodeTabMarker(marker)
    if (
        !decoded ||
        !isSigningAction(action) ||
        (turn !== "payload" && !(turn === "artifact" && hasArtifact(action))) ||
        typeof until !== "number" ||
        until <= Date.now() ||
        until > Date.now() + 10 * 60 * 1000
    )
        return undefined
    return {marker: decoded, action, turn}
}

export function saveProductTab(storage: Storage, tab: ProductTab) {
    const value = {...tab, until: Date.now() + 10 * 60 * 1000}
    if (!decodeProductTab(value)) return false
    try {
        storage.setItem(TAB, JSON.stringify(value))
        return true
    } catch {
        return false
    }
}

export function takeProductTab(storage: Storage): ProductTab | undefined {
    try {
        const value = storage.getItem(TAB)
        storage.removeItem(TAB)
        return value && value.length <= 2048 ? decodeProductTab(JSON.parse(value)) : undefined
    } catch {
        return undefined
    }
}

export function parseProductReply(value: unknown, expected: ProductApprovalScope): ProductReply {
    const data = record(value)
    for (const key of ["tenant", "event", "request", "actor", "action"] as const) {
        if (data[key] !== expected[key]) throw new Error("Approval did not match")
    }
    if (
        data.phase === "gold" &&
        isId(data.attempt) &&
        typeof data.authorization_url === "string" &&
        data.authorization_url.length <= 2048
    ) {
        return {phase: "gold", attempt: data.attempt, authorization_url: data.authorization_url}
    }
    if (data.phase === "recorded" || data.phase === "reconcile" || data.phase === "ready")
        return {phase: data.phase}
    throw new Error("Approval state unavailable")
}

export function productionUrl(value: string): URL {
    const url = new URL(value)
    const host = url.hostname.toLowerCase().replace(/\.$/, "")
    if (
        url.protocol !== "https:" ||
        url.username ||
        url.password ||
        url.hash ||
        host === "localhost" ||
        host.endsWith(".localhost") ||
        /^127\./.test(host) ||
        host === "0.0.0.0" ||
        host === "[::1]" ||
        host === "[::]" ||
        /^\[::ffff:7f[0-9a-f]{2}:/.test(host) ||
        host === "[::ffff:0:0]"
    ) {
        throw new Error("Production signing requires HTTPS")
    }
    return url
}

export function productGoldRedirect(
    raw: string,
    keycloak: string,
    goldClient: string,
    tenant: string
) {
    if (!CLIENT.test(goldClient) || !isId(tenant))
        throw new Error("Production signing is unavailable")
    const base = productionUrl(keycloak)
    const url = productionUrl(raw)
    const auth =
        base.pathname.replace(/\/$/, "") + `/realms/tenant-${tenant}/protocol/openid-connect/auth`
    const par = url.searchParams.get("request_uri")
    if (
        base.search ||
        url.origin !== base.origin ||
        url.pathname !== auth ||
        Array.from(url.searchParams).length !== 2 ||
        url.searchParams.get("client_id") !== goldClient ||
        !par ||
        !/^urn:ietf:params:oauth:request_uri:[a-zA-Z0-9_.-]{1,256}$/.test(par)
    )
        throw new Error("Invalid production verification redirect")
    return url.href
}

export function matchesProductNormalSession(
    claims: unknown,
    scope: ProductApprovalScope,
    issuer: string,
    client: string,
    audience: string
) {
    const token = record(claims)
    const hasura = record(token["https://hasura.io/jwt/claims"])
    const roles = hasura["x-hasura-allowed-roles"]
    return (
        token.iss === issuer &&
        token.azp === client &&
        (token.aud === audience || (Array.isArray(token.aud) && token.aud.includes(audience))) &&
        token.typ === "Bearer" &&
        token.sub === scope.actor &&
        hasura["x-hasura-user-id"] === scope.actor &&
        hasura["x-hasura-tenant-id"] === scope.tenant &&
        Array.isArray(roles) &&
        roles.includes(SIGNING_ACTIONS[scope.action].signPermission)
    )
}

export async function productRequest(
    origin: string,
    bearer: string,
    scope: ProductApprovalScope,
    input:
        | {step: "start"; reviewedPayload: string}
        | {step: "finish"; attempt: string; state: string; code: string}
        | {step: "reconcile"}
) {
    const base = productionUrl(origin)
    if (base.search || base.pathname !== "/" || !isId(scope.tenant) || !isId(scope.request))
        throw new Error("Approval scope unavailable")
    if (!bearer || bearer.length > 32768 || !/^[A-Za-z0-9._-]+$/.test(bearer))
        throw new Error("Approval session unavailable")
    const url = new URL(`/operational-signing/v2/${scope.tenant}/${input.step}`, base)
    const payload =
        input.step === "start"
            ? {request_id: scope.request, reviewed_payload_sha256: input.reviewedPayload}
            : input.step === "finish"
              ? {
                    request_id: scope.request,
                    attempt: input.attempt,
                    state: input.state,
                    code: input.code,
                }
              : {request_id: scope.request}
    const response = await fetch(url, {
        method: "POST",
        credentials: "omit",
        redirect: "error",
        referrerPolicy: "no-referrer",
        cache: "no-store",
        signal: AbortSignal.timeout(15000),
        headers: {"Content-Type": "application/json", "Authorization": "Bearer " + bearer},
        body: JSON.stringify(payload),
    })
    if (!response.ok || response.url !== url.href) throw new Error("Approval status unavailable")
    return parseProductReply(await response.json(), scope)
}
