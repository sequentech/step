// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import Keycloak from "keycloak-js"
import {
    INITIALIZE_RETURN_PATH,
    localInitializeBuild,
    matchLocalInitializeSettings,
} from "./localInitializeAuthority"
import {
    bindReturnedPage,
    clearTabMarker,
    readTabMarker,
    saveDisplayReturn,
    signingReturnPath,
    type ITabMarker,
    type ReturnSecret,
    type VisibleResult,
} from "./operationalGoldReturn"

const record = (value: unknown): Record<string, unknown> =>
    value && typeof value === "object" && !Array.isArray(value)
        ? (value as Record<string, unknown>)
        : {}

export function matchesNormalSessionActor(
    claims: unknown,
    tab: ITabMarker,
    issuer: string,
    client: string
) {
    const token = record(claims)
    const hasura = record(token["https://hasura.io/jwt/claims"])
    const audience =
        token.aud === client || (Array.isArray(token.aud) && token.aud.includes(client))
    return (
        token.iss === issuer &&
        token.azp === client &&
        audience &&
        token.sub === tab.official &&
        hasura["x-hasura-tenant-id"] === tab.tenant &&
        hasura["x-hasura-user-id"] === tab.official &&
        Array.isArray(hasura["x-hasura-allowed-roles"]) &&
        hasura["x-hasura-allowed-roles"].includes("sign-initialize-voting")
    )
}

const secureUrl = (url: URL) =>
    url.protocol === "https:" && !url.username && !url.password && !url.search && !url.hash

async function silentNormalSession(
    browser: Window,
    config: {issuer: URL; client: string},
    tab: ITabMarker
) {
    const adapter = new Keycloak({
        url: config.issuer.href,
        realm: "tenant-" + tab.tenant,
        clientId: config.client,
    })
    // Guard the public adapter against unsolicited top-level auth on this dedicated callback.
    adapter.login = async () => {
        throw new Error("Silent session unavailable")
    }
    let timer: number | undefined
    const expectedIssuer = config.issuer.href.replace(/\/$/, "") + "/realms/tenant-" + tab.tenant
    const initialize = adapter.init({
        onLoad: "check-sso",
        silentCheckSsoRedirectUri: browser.location.origin + "/silent-check-sso.html",
        silentCheckSsoFallback: false,
        checkLoginIframe: false,
        enableLogging: false,
        responseMode: "fragment",
        flow: "standard",
        pkceMethod: "S256",
        messageReceiveTimeout: 6500,
    })
    let timedOut = false
    try {
        const ok = await Promise.race([
            initialize,
            new Promise<false>((resolve) => {
                timer = browser.setTimeout(() => {
                    timedOut = true
                    resolve(false)
                }, 8500)
            }),
        ])
        if (
            !ok ||
            !matchesNormalSessionActor(adapter.tokenParsed, tab, expectedIssuer, config.client)
        )
            return
        return adapter.token
    } finally {
        browser.clearTimeout(timer)
        if (timedOut) void initialize.finally(() => adapter.clearToken()).catch(() => undefined)
        else adapter.clearToken()
    }
}

/** Exchange the one-use proof only from an independently authenticated Admin session.
 * On any failure, return to the original request to check its durable status, never to retry.
 */
export async function finishOperationalApproval(
    browser: Window,
    captured: ReturnSecret | undefined
) {
    const location = browser.location
    const local = localInitializeBuild(location.origin)
    if (
        location.protocol !== "https:" &&
        !(
            local &&
            location.protocol === "http:" &&
            location.href === location.origin + INITIALIZE_RETURN_PATH
        )
    )
        return false
    const storage = browser.sessionStorage
    const tab = readTabMarker(storage)
    if (!tab) return false
    clearTabMarker(storage)
    const resume = (result: VisibleResult) => {
        const path = signingReturnPath(tab.event, tab.page)
        saveDisplayReturn(storage, {marker: tab, path, result})
        location.replace(path)
        return true
    }
    const setup = tab.purpose === "initialize-setup"
    if (!captured || (setup && !local)) return resume("reconcile")
    try {
        const settingsUrl = new URL("/global-settings.json", location.origin)
        const settings = await fetch(settingsUrl, {
            cache: "no-store",
            redirect: "error",
            credentials: local ? "omit" : "same-origin",
            referrerPolicy: "no-referrer",
            signal: AbortSignal.timeout(4000),
        })
        if (!settings.ok || settings.url !== settingsUrl.href) return resume("reconcile")
        const config = record(await settings.json())
        const {KEYCLOAK_URL, HASURA_URL, ONLINE_VOTING_CLIENT_ID} = config
        if (
            typeof KEYCLOAK_URL !== "string" ||
            typeof HASURA_URL !== "string" ||
            typeof ONLINE_VOTING_CLIENT_ID !== "string" ||
            !/^[a-zA-Z0-9_-]{1,120}$/.test(ONLINE_VOTING_CLIENT_ID) ||
            ONLINE_VOTING_CLIENT_ID.startsWith("sequent-operational-")
        )
            return resume("reconcile")
        const issuer = new URL(KEYCLOAK_URL)
        const graph = new URL(HASURA_URL, location.origin)
        if (local) {
            const registration = matchLocalInitializeSettings(location.origin, tab.tenant, config)
            if (
                !registration ||
                captured.issuer !== registration.issuer ||
                (setup && !registration.setupClient)
            )
                return resume("reconcile")
        } else if (
            !secureUrl(issuer) ||
            !secureUrl(graph) ||
            graph.origin !== location.origin ||
            graph.pathname !== "/v1/graphql"
        )
            return resume("reconcile")

        const token = await silentNormalSession(
            browser,
            {issuer, client: ONLINE_VOTING_CLIENT_ID},
            tab
        )
        if (!token) return resume("reconcile")
        const response = await fetch(graph, {
            method: "POST",
            cache: "no-store",
            redirect: "error",
            credentials: local ? "omit" : "same-origin",
            referrerPolicy: "no-referrer",
            signal: AbortSignal.timeout(12000),
            headers: {
                "Content-Type": "application/json",
                "Authorization": "Bearer " + token,
                "x-hasura-role": "sign-initialize-voting",
            },
            body: JSON.stringify({
                operationName: setup ? "LocalInitializeSetupReturn" : "OperationalGoldReturn",
                query: setup
                    ? `mutation LocalInitializeSetupReturn($request: uuid!, $digest: String!, $attempt: uuid!, $state: String!, $code: String!) {
                    operationalInitializeSelfSetupConfirm(request_id: $request, payload_sha256: $digest, attempt: $attempt, state: $state, code: $code) {
                        tenant event official request attempt phase page {path panel election_id}
                    }
                }`
                    : `mutation OperationalGoldReturn($request: uuid!, $attempt: uuid!, $state: String!, $code: String!) {
                    operationalInitializeFinish(request_id: $request, attempt: $attempt, state: $state, code: $code) {
                        tenant event official request attempt phase page {path panel election_id}
                    }
                }`,
                variables: {
                    request: tab.request,
                    attempt: tab.attempt,
                    ...(setup && {digest: tab.payloadSha256}),
                    state: captured.state,
                    code: captured.code,
                },
            }),
        })
        if (!response.ok || (local && response.url !== graph.href)) return resume("reconcile")
        const body = record(await response.json())
        const returned = record(
            record(body.data)[
                setup ? "operationalInitializeSelfSetupConfirm" : "operationalInitializeFinish"
            ]
        )
        if (
            body.errors ||
            !bindReturnedPage(tab, returned, {tenant: tab.tenant, official: tab.official})
        )
            return resume("reconcile")
        if (setup)
            return resume(
                returned.phase === "setup-confirmation-required" ? "setup-returned" : "reconcile"
            )
        return resume(returned.phase === "approval-recorded" ? "approval-recorded" : "reconcile")
    } catch {
        return resume("reconcile")
    }
}
