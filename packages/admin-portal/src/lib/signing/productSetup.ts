// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {isId} from "./operationalGoldReturn"

export type ProductSetupScope = {
    tenant: string
    event: string
    request: string
    reviewedPayload: string
}
const setupPhases = [
    "awaiting-independent-officer",
    "hardware-generation-pending",
    "certificate-pending",
    "certificate-on-file",
    "reconciliation-required",
] as const
export type ProductSetupPhase = (typeof setupPhases)[number]

export type ProductSetupResult = {
    ticket: string
    tenant: string
    event: string
    request: string
    official: string
    phase: ProductSetupPhase
}

/** A setup intent, not a signature or authority to make one. Same-origin
 * pinned product transport only. Don't retry a mutation on an unknown reply. */
export async function requestProductSetup(
    token: string,
    scope: ProductSetupScope,
    official: string,
    signal?: AbortSignal
): Promise<ProductSetupResult> {
    const origin = window.location.origin
    if (
        window.location.protocol !== "https:" ||
        !token ||
        ![scope.tenant, scope.event, scope.request, official].every(isId) ||
        !/^[0-9a-f]{64}$/.test(scope.reviewedPayload)
    ) {
        throw new Error("Hardware setup scope unavailable")
    }
    const result = await fetch(new URL(`/operational-signing/v2/${scope.tenant}/setup`, origin), {
        method: "POST",
        credentials: "omit",
        mode: "same-origin",
        redirect: "error",
        cache: "no-store",
        headers: {"Content-Type": "application/json", "Authorization": `Bearer ${token}`},
        body: JSON.stringify({
            request_id: scope.request,
            reviewed_payload_sha256: scope.reviewedPayload,
        }),
        signal,
    })
    if (!result.ok) throw new Error("Hardware setup was not confirmed")
    const data: unknown = await result.json()
    if (!data || typeof data !== "object") throw new Error("Hardware setup reply is invalid")
    const reply = data as Partial<ProductSetupResult>
    if (
        !isId(reply.ticket) ||
        reply.tenant !== scope.tenant ||
        reply.event !== scope.event ||
        reply.request !== scope.request ||
        reply.official !== official ||
        !reply.phase ||
        !setupPhases.includes(reply.phase)
    ) {
        throw new Error("Hardware setup reply is out of scope")
    }
    return {
        ticket: reply.ticket,
        tenant: reply.tenant,
        event: reply.event,
        request: reply.request,
        official: reply.official,
        phase: reply.phase,
    }
}
