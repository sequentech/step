// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import Keycloak from "keycloak-js"
import {saveDisplayReturn, signingReturnPath} from "./operationalGoldReturn"
import type {ProductCode} from "./productCallbackCapture"
import {
    hasArtifact,
    matchesProductNormalSession,
    productionUrl,
    productGoldRedirect,
    productRequest,
    record,
    saveProductTab,
    scopeOf,
    takeProductTab,
    type ProductApprovalScope,
} from "./productApproval"

async function normalToken(
    browser: Window,
    scope: ProductApprovalScope,
    issuer: URL,
    client: string,
    audience: string
) {
    const realm = "tenant-" + scope.tenant
    const expectedIssuer = issuer.href.replace(/\/$/, "") + "/realms/" + realm
    const adapter = new Keycloak({url: issuer.href, realm, clientId: client})
    adapter.login = async () => {
        throw new Error("Normal session unavailable")
    }
    let timedOut = false
    let timer: number | undefined
    const initializing = adapter.init({
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
    try {
        const authenticated = await Promise.race([
            initializing,
            new Promise<false>((resolve) => {
                timer = browser.setTimeout(() => {
                    timedOut = true
                    resolve(false)
                }, 8500)
            }),
        ])
        if (
            authenticated &&
            matchesProductNormalSession(
                adapter.tokenParsed,
                scope,
                expectedIssuer,
                client,
                audience
            )
        ) {
            return adapter.token
        }
    } finally {
        browser.clearTimeout(timer)
        if (timedOut) void initializing.finally(() => adapter.clearToken()).catch(() => undefined)
        else adapter.clearToken()
    }
}

/** The captured code and normal session token only exist in this short-lived page instance. */
export async function finishProductApproval(browser: Window, secret: ProductCode | undefined) {
    const location = browser.location
    if (location.protocol !== "https:") return false
    const storage = browser.sessionStorage
    const tab = takeProductTab(storage)
    if (!tab) return false
    const resume = (result: "approval-recorded" | "reconcile") => {
        const path = signingReturnPath(tab.marker.event, tab.marker.page)
        saveDisplayReturn(storage, {marker: tab.marker, path, result})
        location.replace(path)
        return true
    }
    if (!secret) return resume("reconcile")
    try {
        const realmSettings = new URL("/global-settings.json", productionUrl(location.origin))
        const settings = await fetch(realmSettings, {
            cache: "no-store",
            redirect: "error",
            credentials: "same-origin",
            referrerPolicy: "no-referrer",
            signal: AbortSignal.timeout(4000),
        })
        if (!settings.ok || settings.url !== realmSettings.href) return resume("reconcile")
        const {
            KEYCLOAK_URL,
            ONLINE_VOTING_CLIENT_ID,
            OPERATIONAL_PRODUCT_SIGNING_GOLD_CLIENT_ID: goldClient,
            OPERATIONAL_PRODUCT_NORMAL_AUDIENCE: audience,
        } = record(await settings.json())
        if (
            typeof KEYCLOAK_URL !== "string" ||
            typeof ONLINE_VOTING_CLIENT_ID !== "string" ||
            !/^[a-zA-Z0-9_.-]{1,128}$/.test(ONLINE_VOTING_CLIENT_ID) ||
            typeof goldClient !== "string" ||
            !/^[a-zA-Z0-9_.-]{1,128}$/.test(goldClient) ||
            goldClient === ONLINE_VOTING_CLIENT_ID ||
            typeof audience !== "string" ||
            !/^[a-zA-Z0-9_.:/-]{1,256}$/.test(audience)
        )
            return resume("reconcile")
        const issuer = productionUrl(KEYCLOAK_URL)
        if (issuer.search) return resume("reconcile")
        const scope = scopeOf(tab)
        const expectedIssuer = issuer.href.replace(/\/$/, "") + "/realms/tenant-" + scope.tenant
        if (secret.issuer !== undefined && secret.issuer !== expectedIssuer)
            return resume("reconcile")
        const token = await normalToken(browser, scope, issuer, ONLINE_VOTING_CLIENT_ID, audience)
        if (!token) return resume("reconcile")
        const result = await productRequest(location.origin, token, scope, {
            step: "finish",
            attempt: tab.marker.attempt,
            ...secret,
        })
        if (result.phase === "gold") {
            if (
                tab.turn !== "payload" ||
                !hasArtifact(tab.action) ||
                result.attempt === tab.marker.attempt
            )
                return resume("reconcile")
            // The document leg has its own exact server-only intent, state and Gold authentication.
            const next = productGoldRedirect(
                result.authorization_url,
                KEYCLOAK_URL,
                goldClient,
                scope.tenant
            )
            if (
                !saveProductTab(storage, {
                    ...tab,
                    marker: {...tab.marker, attempt: result.attempt},
                    turn: "artifact",
                })
            )
                return resume("reconcile")
            location.replace(next)
            return true
        }
        return resume(result.phase === "recorded" ? "approval-recorded" : "reconcile")
    } catch {
        // Never submit the same code again or retry an irreversible device operation.
        return resume("reconcile")
    }
}
