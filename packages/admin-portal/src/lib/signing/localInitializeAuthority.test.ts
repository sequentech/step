// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {randomUUID} from "node:crypto"
import profile from "./localInitializeProfile.json"
import {localInitializeRegistration, matchLocalInitializeSettings} from "./localInitializeAuthority"
import {createOperationalApprovalApi, trustedSigningRedirect} from "./operationalApprovalApi"
import {captureGoldReturn} from "./operationalGoldReturn"
import {createLocalInitializeSetupApi, localSetupStatus, localDialogPhase} from "./localInitializeSetupApi"

jest.mock("./localInitializeProfile.json", () => ({profile: "disabled"}))

const origin = "http://localhost:3002"
const base = "http://localhost:18090/"
const tenant = randomUUID()
const issuer = base + "realms/tenant-" + tenant
const callback = origin + "/operational-signing/return"
const graphql = "http://localhost:18080/v1/graphql"
const install = {profile: "felix3-local-initialize", deviceProfile: "yubihsm", issuer, callback, graphql,
    portalClient: "portal-test", approvalClient: "approve-test"}
const settings = {KEYCLOAK_URL: base, HASURA_URL: graphql, ONLINE_VOTING_CLIENT_ID: install.portalClient,
    FELIX3_LOCAL_INITIALIZE: install}
const endpoint = issuer + "/protocol/openid-connect/auth"
const query = new URLSearchParams({
    client_id: install.approvalClient, request_uri: "urn:ietf:params:oauth:request_uri:test",
}).toString()
const redirect = endpoint + "?" + query

describe("ordinary local Initialize registration", () => {
    beforeEach(() => {
        profile.profile = "felix3-local-initialize"
    })
    afterEach(() => {
        profile.profile = "disabled"
    })

    it("requires both the selected bundle and the separate installed registration", () => {
        expect(jest.requireActual("./localInitializeProfile.json")).toEqual({profile: "disabled"})
        expect(matchLocalInitializeSettings(origin, tenant, settings)?.issuer).toBe(issuer)
        expect(localInitializeRegistration(origin)).toBeUndefined()
        expect(() => trustedSigningRedirect(redirect, base, tenant, origin)).toThrow()
        profile.profile = "disabled"
        expect(matchLocalInitializeSettings(origin, tenant, settings)).toBeUndefined()
        expect(() => trustedSigningRedirect(redirect, base, tenant, origin, install)).toThrow()
        expect(() => createOperationalApprovalApi({query: jest.fn(), mutate: jest.fn()}, origin, install)).toThrow()
    })

    it("compares the normal app settings and the independent registration byte for byte", () => {
        for (const address of [origin + "/", "http://127.0.0.1:3002", "http://192.0.2.1:3002",
            "http://localhost.:3002", "http://localhost:3003", "http://localhost:3002.example"]) {
            expect(matchLocalInitializeSettings(address, tenant, settings)).toBeUndefined()
        }
        for (const patch of [
            {KEYCLOAK_URL: "http://localhost:8090/"}, {KEYCLOAK_URL: "http://keycloak:8090/"},
            {KEYCLOAK_URL: base + "else/../"}, {HASURA_URL: "http://localhost:8080/v1/graphql"},
            {HASURA_URL: graphql + "?"}, {ONLINE_VOTING_CLIENT_ID: "different"},
        ]) expect(matchLocalInitializeSettings(origin, tenant, {...settings, ...patch})).toBeUndefined()
        expect(matchLocalInitializeSettings(origin, randomUUID(), settings)).toBeUndefined()
        for (const patch of [
            {issuer: issuer.replace("18090", "8090")}, {issuer: issuer.replace("/realms", "/a/../realms")},
            {issuer: issuer + "#"}, {callback: callback + "/"}, {graphql: graphql + "?q=x"},
            {approvalClient: install.portalClient}, {deviceProfile: "softhsm"},
            {credentials: "include"}, {tokenEndpoint: "http://elsewhere.invalid/token"},
        ]) {
            expect(localInitializeRegistration(origin, {...install, ...patch})).toBeUndefined()
        }
    })

    it("only follows the broker's exact public PAR URL for this registered client", () => {
        expect(trustedSigningRedirect(redirect, base, tenant, origin, install)).toBe(redirect)
        for (const raw of [
            redirect.replace("localhost:18090", "keycloak:8090"),
            redirect.replace("localhost:", "@localhost:"), redirect.replace("18090", "018090"),
            redirect.replace("/realms", "/x/%2e%2e/realms"), redirect.replace(tenant, randomUUID()),
            redirect.replace(install.approvalClient, install.portalClient), redirect + "#",
            redirect + "&client_id=" + install.approvalClient, redirect + "&redirect_uri=x",
        ]) expect(() => trustedSigningRedirect(raw, base, tenant, origin, install)).toThrow()
    })

    it("requires the separate installed software setup client and its exact PAR redirect", () => {
        const setupClient = "felix3-local-initialize-setup"
        const software = {...install, deviceProfile: "softhsm-test", setupClient}
        const par = redirect.replace(install.approvalClient, setupClient)
        const call = (url: string, reg: unknown) => trustedSigningRedirect(url, base, tenant, origin, reg, "initialize-setup")
        expect(call(par, software)).toBe(par)
        for (const reg of [install, {...software, setupClient: install.portalClient},
            {...software, approvalClient: setupClient}, {...software, deviceProfile: "yubihsm"}]) {
            expect(createLocalInitializeSetupApi({query: jest.fn(), mutate: jest.fn()}, origin, reg)).toBeUndefined()
            expect(() => call(par, reg)).toThrow()
        }
        for (const url of [redirect, par.replace("localhost:18090", "keycloak:8090"), par + "&code=test", par + "%0A"]) {
            expect(() => call(url, software)).toThrow()
        }
        expect(() => trustedSigningRedirect(par, base, tenant, origin, software)).toThrow()
        profile.profile = "disabled"
        expect(() => call(par, software)).toThrow()
    })

    it("requires an owned attempt and delivery revision for registration and separate approval eligibility", () => {
        const scope = {tenant, event: randomUUID(), request: randomUUID(), official: randomUUID()}
        const attempt = randomUUID()
        const result = {scope, device_profile: "softhsm-test", phase: "setup-confirmation-required", attempt, delivery_revision: 0}
        expect(localSetupStatus(result, scope)).toEqual({phase: result.phase, attempt, revision: 0})
        for (const change of [{attempt: null}, {delivery_revision: -1}, {delivery_revision: null},
            {device_profile: "yubihsm"}, {phase: "setup-pending"}, {scope: {...scope, official: randomUUID()}}]) {
            expect(() => localSetupStatus({...result, ...change}, scope)).toThrow()
        }
        expect(localDialogPhase("challenge-available", "setup-pending")).toBe("setup-pending")
        expect(localDialogPhase("unavailable", "certificate-on-file")).toBe("certificate-on-file")
        expect(localDialogPhase("challenge-available", "certificate-on-file")).toBe("challenge-available")
    })

    it("scrubs a return even when its destination or response issuer is refused", () => {
        const state = "n".repeat(43)
        const clean = jest.fn()
        const capture = (destination: string, returnedIssuer: string) => {
            const hash = "#" + new URLSearchParams({code: "opaque", state, iss: returnedIssuer})
            return captureGoldReturn({origin, protocol: "http:", pathname: "/operational-signing/return",
                href: destination + hash, hash, search: ""}, clean)
        }
        expect(capture(callback, issuer)).toEqual({state, code: "opaque", issuer})
        expect(capture(callback, issuer.replace("localhost:18090", "keycloak:8090"))).toBeUndefined()
        expect(capture(callback + "?", issuer)).toBeUndefined()
        expect(capture(callback.replace("localhost", "x@localhost"), issuer)).toBeUndefined()
        expect(clean).toHaveBeenCalledTimes(4)
    })

    it("labels software only for an explicit local software registration and matching status", async () => {
        const software = {...install, deviceProfile: "softhsm-test"}
        profile.profile = "disabled"
        expect(localInitializeRegistration(origin, software)).toBeUndefined()
        profile.profile = "felix3-local-initialize"
        const scope = {tenant, request: randomUUID(), official: randomUUID(), event: randomUUID()}
        const query = jest.fn()
        const api = createOperationalApprovalApi({query, mutate: jest.fn()}, origin, software)
        const answer = (device_profile: string) => ({data: {operationalInitializeStatus: {
            scope, phase: "challenge-available", device_profile,
        }}})
        query.mockResolvedValueOnce(answer("softhsm-test"))
        await expect(api.status(scope)).resolves.toMatchObject({softwareDemo: true})
        query.mockResolvedValueOnce(answer("yubihsm"))
        await expect(api.status(scope)).rejects.toThrow("Signing device unavailable")
    })

    it("uses the exact GraphQL destination without cookies and refuses software device status", async () => {
        const scope = {tenant, request: randomUUID(), official: randomUUID(), event: randomUUID()}
        const query = jest.fn().mockResolvedValue({data: {operationalInitializeStatus: {
            scope, phase: "challenge-available", device_profile: "softhsm-test",
        }}})
        const mutate = jest.fn()
        const api = createOperationalApprovalApi({query, mutate}, origin, install)
        await expect(api.setupStatus(scope)).rejects.toThrow("Setup unavailable")
        await expect(api.setup(scope)).rejects.toThrow("Setup unavailable")
        expect(mutate).not.toHaveBeenCalled()
        expect(query).not.toHaveBeenCalled()
        await expect(api.status(scope)).rejects.toThrow("Signing device unavailable")
        expect(query).toHaveBeenCalledWith(expect.objectContaining({context: {
            uri: graphql, headers: {"x-hasura-role": "sign-initialize-voting"}, credentials: "omit",
            fetchOptions: {redirect: "error", credentials: "omit", referrerPolicy: "no-referrer"},
        }}))
    })
})
