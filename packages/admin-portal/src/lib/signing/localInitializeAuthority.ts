// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import buildProfile from "./localInitializeProfile.json"

export const INITIALIZE_RETURN_PATH = "/operational-signing/return"
export const LOCAL_SETUP_CLIENT = "felix3-local-initialize-setup"
const PORTAL = "http://localhost:3002"
const KEYCLOAK = "http://localhost:18090/"
const GRAPHQL = "http://localhost:18080/v1/graphql"
const registrationFields = new Set([
    "profile",
    "deviceProfile",
    "issuer",
    "callback",
    "graphql",
    "portalClient",
    "approvalClient",
    "setupClient",
])
const clientId = (s: unknown): s is string =>
    typeof s === "string" && /^[A-Za-z0-9_.-]{1,150}$/.test(s) && !/\s/.test(s)

/** Static bundle selection. The ordinary optimized artifact always ships disabled. */
export function localInitializeBuild(origin: string | undefined) {
    return origin === PORTAL && buildProfile.profile === "felix3-local-initialize"
}

// The operator publishes only these public registration values in the normal
// app settings. A disabled bundle cannot consume them.
export function localInitializeRegistration(origin: string | undefined, registration?: unknown) {
    if (!localInitializeBuild(origin) || !registration || typeof registration !== "object")
        return undefined
    const p = registration as Record<string, unknown>
    const deviceProfile = p.deviceProfile
    if (
        Object.keys(p).some((key) => !registrationFields.has(key)) ||
        p.profile !== buildProfile.profile ||
        (deviceProfile !== "yubihsm" && deviceProfile !== "softhsm-test") ||
        p.callback !== PORTAL + INITIALIZE_RETURN_PATH ||
        p.graphql !== GRAPHQL ||
        !clientId(p.portalClient) ||
        p.portalClient.length > 120 ||
        p.portalClient.includes(".") ||
        p.portalClient.startsWith("sequent-operational-") ||
        !clientId(p.approvalClient) ||
        p.portalClient === p.approvalClient ||
        p.portalClient === LOCAL_SETUP_CLIENT ||
        p.approvalClient === LOCAL_SETUP_CLIENT ||
        (p.setupClient !== undefined &&
            (p.setupClient !== LOCAL_SETUP_CLIENT || deviceProfile !== "softhsm-test")) ||
        p.approvalClient === "sequent-operational-demo-14990" ||
        typeof p.issuer !== "string" ||
        p.issuer.length > 1024 ||
        !p.issuer.startsWith(KEYCLOAK + "realms/tenant-")
    )
        return undefined
    const tenant = p.issuer.slice((KEYCLOAK + "realms/tenant-").length)
    // Check the raw spelling before a URL parser can remove dot segments,
    // userinfo, whitespace or an alternate spelling of the authority.
    if (
        tenant.length !== 36 ||
        !/^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/.test(tenant) ||
        tenant === "00000000-0000-0000-0000-000000000000"
    )
        return undefined
    return {
        issuer: p.issuer,
        deviceProfile,
        tenant,
        callback: PORTAL + INITIALIZE_RETURN_PATH,
        graphql: GRAPHQL,
        portalClient: p.portalClient,
        approvalClient: p.approvalClient,
        setupClient: p.setupClient === LOCAL_SETUP_CLIENT ? LOCAL_SETUP_CLIENT : undefined,
    }
}

/** Bind the independently installed realm/client to the normal app's settings. */
export function matchLocalInitializeSettings(
    origin: string,
    tenant: string,
    settings: {
        KEYCLOAK_URL?: unknown
        HASURA_URL?: unknown
        ONLINE_VOTING_CLIENT_ID?: unknown
        FELIX3_LOCAL_INITIALIZE?: unknown
    }
) {
    const local = localInitializeRegistration(origin, settings.FELIX3_LOCAL_INITIALIZE)
    if (
        !local ||
        tenant !== local.tenant ||
        settings.KEYCLOAK_URL !== KEYCLOAK ||
        settings.HASURA_URL !== local.graphql ||
        settings.ONLINE_VOTING_CLIENT_ID !== local.portalClient
    )
        return undefined
    return local
}
