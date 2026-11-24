// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    markBeforeFullRedirect,
    readDisplayReturn,
    readTabMarker,
    type ITabMarker,
} from "./operationalGoldReturn"
import localProfile from "./localInitializeProfile.json"

jest.mock("./localInitializeProfile.json", () => ({profile: "disabled"}))

jest.mock("keycloak-js", () => ({
    __esModule: true,
    default: function () {
        return mockAdapter
    },
}))
const id = (last: number) => "00000000-0000-4000-8000-00000000000" + last
const tab: ITabMarker = {
    tenant: id(1),
    event: id(2),
    request: id(3),
    official: id(4),
    attempt: id(5),
    page: {path: "election-show", panel: "publish", election_id: id(6)},
}
const path = "/sequent_backend_election/" + id(6) + "/show"
const origin = "https://127.0.0.1:13443"
const keycloak = "https://127.0.0.1:18443"
const issuer = keycloak + "/realms/tenant-" + tab.tenant
const client = "admin-portal"
const claims = {
    "iss": issuer,
    "azp": client,
    "aud": client,
    "sub": tab.official,
    "https://hasura.io/jwt/claims": {
        "x-hasura-tenant-id": tab.tenant,
        "x-hasura-user-id": tab.official,
        "x-hasura-allowed-roles": ["sign-initialize-voting"],
    },
}
const mockAdapter = {
    init: jest.fn<Promise<boolean>, []>(),
    login: jest.fn(),
    clearToken: jest.fn(),
    tokenParsed: claims,
    token: "synthetic-admin-token",
}
const {finishOperationalApproval, matchesNormalSessionActor} =
    require("./finishOperationalApproval") as typeof import("./finishOperationalApproval")
const proof = {state: "state-only-in-memory", code: "code-only-in-memory"}
const settings = {
    KEYCLOAK_URL: keycloak,
    HASURA_URL: origin + "/v1/graphql",
    ONLINE_VOTING_CLIENT_ID: client,
}
const fetchStub = jest.fn()
const response = (json: unknown, url = "", ok = true) => ({ok, url, json: async () => json})

function browser(portalOrigin = origin) {
    const store = new Map<string, string>()
    const storage = {
        getItem: (key: string) => store.get(key) ?? null,
        setItem: (key: string, value: string) => void store.set(key, value),
        removeItem: (key: string) => void store.delete(key),
    } as Storage
    const location = {protocol: new URL(portalOrigin).protocol, origin: portalOrigin,
        href: portalOrigin + "/operational-signing/return", replace: jest.fn()}
    const candidate = {
        location,
        sessionStorage: storage,
        setTimeout,
        clearTimeout,
    } as unknown as Window
    return {candidate, storage, location, store}
}

describe("authenticated Gold return", () => {
    beforeEach(() => {
        jest.clearAllMocks()
        fetchStub.mockReset()
        mockAdapter.tokenParsed = claims
        mockAdapter.init.mockResolvedValue(true)
        jest.spyOn(global, "fetch").mockImplementation(fetchStub)
    })
    afterEach(() => jest.restoreAllMocks())

    const install = (
        returned: unknown = {...tab, phase: "approval-recorded"},
        config = settings
    ) => {
        const b = browser()
        expect(markBeforeFullRedirect(b.storage, tab)).toBe(true)
        fetchStub.mockResolvedValueOnce(response(config, origin + "/global-settings.json"))
        fetchStub.mockResolvedValueOnce(response({data: {operationalInitializeFinish: returned}}))
        return b
    }

    it("checks the normal Portal actor before exchanging one proof and restores the exact request", async () => {
        const b = install()
        expect(await finishOperationalApproval(b.candidate, proof)).toBe(true)
        expect(mockAdapter.init).toHaveBeenCalledWith(
            expect.objectContaining({
                onLoad: "check-sso",
                pkceMethod: "S256",
                silentCheckSsoFallback: false,
            })
        )
        expect(mockAdapter.clearToken).toHaveBeenCalledTimes(1)
        expect(fetchStub).toHaveBeenCalledTimes(2)
        const [destination, request] = fetchStub.mock.calls[1]
        expect(String(destination)).toBe(origin + "/v1/graphql")
        expect(request.headers.Authorization).toBe("Bearer synthetic-admin-token")
        expect(JSON.parse(request.body).variables).toEqual({
            request: tab.request,
            attempt: tab.attempt,
            ...proof,
        })
        expect(b.location.replace).toHaveBeenCalledWith(path)
        expect(
            readDisplayReturn(b.storage, path, {tenant: tab.tenant, official: tab.official})?.result
        ).toBe("approval-recorded")
        expect(readTabMarker(b.storage)).toBeUndefined()
        for (const saved of Array.from(b.store.values())) {
            expect(saved).not.toContain(proof.state)
            expect(saved).not.toContain(proof.code)
        }
        expect(await finishOperationalApproval(b.candidate, proof)).toBe(false)
        expect(fetchStub).toHaveBeenCalledTimes(2)
    })

    it("resumes read-only reconciliation if the fragment is missing", async () => {
        const b = install()
        expect(await finishOperationalApproval(b.candidate, undefined)).toBe(true)
        expect(readDisplayReturn(b.storage, path)?.result).toBe("reconcile")
        expect(fetchStub).not.toHaveBeenCalled()
    })

    it("redeems once from the same normal local actor at the pinned cross-origin Hasura", async () => {
        const portal = "http://localhost:3002"
        const base = "http://localhost:18090/"
        const graph = "http://localhost:18080/v1/graphql"
        const realm = base + "realms/tenant-" + tab.tenant
        const b = browser(portal)
        markBeforeFullRedirect(b.storage, tab)
        expect(await finishOperationalApproval(b.candidate, proof)).toBe(false)
        expect(fetchStub).not.toHaveBeenCalled()
        const localProof = {...proof, issuer: realm}
        try {
            localProfile.profile = "felix3-local-initialize"
            const registration = {profile: "felix3-local-initialize", deviceProfile: "yubihsm", issuer: realm,
                portalClient: client, approvalClient: "synthetic-gold",
                callback: b.location.href, graphql: graph}
            const localSettings = {...settings, KEYCLOAK_URL: base, HASURA_URL: graph,
                FELIX3_LOCAL_INITIALIZE: registration}
            const provideSettings = () =>
                fetchStub.mockResolvedValueOnce(response(localSettings, portal + "/global-settings.json"))

            provideSettings()
            expect(await finishOperationalApproval(b.candidate, {...proof, issuer: base + "realms/tenant-" + id(9)})).toBe(true)
            expect(mockAdapter.init).not.toHaveBeenCalled()
            expect(readDisplayReturn(b.storage, path)?.result).toBe("reconcile")
            markBeforeFullRedirect(b.storage, tab)
            mockAdapter.tokenParsed = {...claims, iss: realm, sub: id(7)}
            provideSettings()
            expect(await finishOperationalApproval(b.candidate, localProof)).toBe(true)
            expect(fetchStub).toHaveBeenCalledTimes(2)
            expect(readDisplayReturn(b.storage, path)?.result).toBe("reconcile")

            markBeforeFullRedirect(b.storage, tab)
            mockAdapter.tokenParsed = {...claims, iss: realm}
            provideSettings()
            fetchStub.mockResolvedValueOnce(response({data: {
                operationalInitializeFinish: {...tab, phase: "approval-recorded"},
            }}, graph))
            expect(await finishOperationalApproval(b.candidate, localProof)).toBe(true)
            expect(readDisplayReturn(b.storage, path)?.result).toBe("approval-recorded")
            expect(await finishOperationalApproval(b.candidate, localProof)).toBe(false)
            expect(fetchStub).toHaveBeenCalledTimes(4)
            expect(String(fetchStub.mock.calls[3][0])).toBe(graph)
            expect(fetchStub.mock.calls[3][1]).toMatchObject({
                credentials: "omit", redirect: "error", referrerPolicy: "no-referrer",
                headers: {Authorization: "Bearer synthetic-admin-token", "x-hasura-role": "sign-initialize-voting"},
            })
        } finally {
            localProfile.profile = "disabled"
        }
    })

    it("never sends the proof to another GraphQL origin even when settings say to", async () => {
        const b = install(tab, {...settings, HASURA_URL: "https://elsewhere.invalid/v1/graphql"})
        expect(await finishOperationalApproval(b.candidate, proof)).toBe(true)
        expect(readDisplayReturn(b.storage, path)?.result).toBe("reconcile")
        expect(fetchStub).toHaveBeenCalledTimes(1)
        expect(mockAdapter.init).not.toHaveBeenCalled()
    })

    it("won't exchange the proof from a normal session for a different official", async () => {
        const b = install()
        mockAdapter.tokenParsed = {...claims, sub: id(7)}
        expect(await finishOperationalApproval(b.candidate, proof)).toBe(true)
        expect(readDisplayReturn(b.storage, path)?.result).toBe("reconcile")
        expect(fetchStub).toHaveBeenCalledTimes(1)
    })

    it("never considers a server return for another Post a recorded approval", async () => {
        const b = install({
            ...tab,
            phase: "approval-recorded",
            page: {...tab.page, election_id: id(7)},
        })
        expect(await finishOperationalApproval(b.candidate, proof)).toBe(true)
        expect(readDisplayReturn(b.storage, path)?.result).toBe("reconcile")
        expect(fetchStub).toHaveBeenCalledTimes(2)
    })

    it("treats transport errors as reconciliation without retries", async () => {
        const b = browser()
        markBeforeFullRedirect(b.storage, tab)
        fetchStub.mockRejectedValueOnce(new TypeError("synthetic network error"))
        expect(await finishOperationalApproval(b.candidate, proof)).toBe(true)
        expect(readDisplayReturn(b.storage, path)?.result).toBe("reconcile")
        expect(fetchStub).toHaveBeenCalledTimes(1)
    })

    it("rejects normal sessions without exact role, tenant, issuer and authorized client", () => {
        expect(matchesNormalSessionActor(claims, tab, issuer, client)).toBe(true)
        expect(
            matchesNormalSessionActor({...claims, aud: [client, "account"]}, tab, issuer, client)
        ).toBe(true)
        for (const token of [
            {...claims, iss: origin},
            {...claims, azp: "alternate"},
            {...claims, aud: ["account"]},
            {
                ...claims,
                "https://hasura.io/jwt/claims": {
                    ...claims["https://hasura.io/jwt/claims"],
                    "x-hasura-tenant-id": id(7),
                },
            },
            {
                ...claims,
                "https://hasura.io/jwt/claims": {
                    ...claims["https://hasura.io/jwt/claims"],
                    "x-hasura-allowed-roles": [],
                },
            },
        ]) {
            expect(matchesNormalSessionActor(token, tab, issuer, client)).toBe(false)
        }
    })
})
