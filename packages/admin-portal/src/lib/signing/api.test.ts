// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {DocumentNode, OperationDefinitionNode} from "graphql"
import {ApolloClient, ApolloLink, InMemoryCache, Observable} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {
    type ISigningApi,
    SigningApiError,
    SigningApiErrorKind,
    createSigningApi,
    toSigningApiError,
    validatePanel,
} from "./api"
import {CertificateCheckId, CertificateOpenFailure, SignatureAlgorithm} from "./types"

/** What Apollo raises for a failed Hasura action forwarding Harvest's answer. */
const actionError = (status: number, body: string) => ({
    message: "http exception when calling webhook",
    graphQLErrors: [
        {
            message: "http exception when calling webhook",
            extensions: {code: "unexpected", internal: {response: {status, body}}},
        },
    ],
})

const operationName = (document: DocumentNode) =>
    (document.definitions[0] as OperationDefinitionNode).name?.value

describe("toSigningApiError", () => {
    it.each([
        ['{"check":"trusted-issuer"}', CertificateCheckId.TrustedIssuer],
        ['{"code":"registered-to-other","message":"x"}', CertificateCheckId.RegisteredToOther],
        ["already-signed", CertificateCheckId.AlreadySigned],
        ['{"extensions":{"code":"post-binding"}}', CertificateCheckId.PostBinding],
        ["something else", null],
    ])("reads the check of a 422 answer %s", (body, check) => {
        const error = toSigningApiError(actionError(422, body))
        expect(error.kind).toBe(SigningApiErrorKind.Refused)
        expect(error.status).toBe(422)
        expect(error.check).toBe(check)
    })

    it.each([
        [409, SigningApiErrorKind.Stale],
        [403, SigningApiErrorKind.Forbidden],
        [401, SigningApiErrorKind.Forbidden],
        [500, SigningApiErrorKind.Other],
    ])("classifies status %s", (status, kind) => {
        expect(toSigningApiError(actionError(status, "")).kind).toBe(kind)
    })

    it("classifies a network failure as other and keeps its message", () => {
        const error = toSigningApiError(new Error("Failed to fetch"))
        expect(error).toBeInstanceOf(SigningApiError)
        expect(error.kind).toBe(SigningApiErrorKind.Other)
        expect(error.message).toBe("Failed to fetch")
    })
})

/** What Hasura answers when Harvest's body carries `extensions.code`. */
const codedError = (code: string, extra: Record<string, unknown> = {}, status = 409) => ({
    message: "refused",
    graphQLErrors: [
        {
            message: "refused",
            extensions: {code, ...extra, internal: {response: {status, body: ""}}},
        },
    ],
})

describe("toSigningApiError by code", () => {
    it.each([
        ["signing-refused", SigningApiErrorKind.Refused],
        ["stale-revision", SigningApiErrorKind.Stale],
        ["already-signed", SigningApiErrorKind.AlreadySigned],
        ["request-closed", SigningApiErrorKind.Closed],
        ["forbidden", SigningApiErrorKind.Forbidden],
    ])("reads %s", (code, kind) => {
        expect(toSigningApiError(codedError(code)).kind).toBe(kind)
    })

    it("reads the refused check from the extensions", () => {
        const error = toSigningApiError(
            codedError("signing-refused", {check: "trusted-issuer"}, 422)
        )
        expect(error.check).toBe(CertificateCheckId.TrustedIssuer)
    })

    // Only stale-revision re-prepares: a 409 with another code is not stale.
    it("prefers the code over the status", () => {
        expect(toSigningApiError(codedError("already-signed", {}, 409)).kind).toBe(
            SigningApiErrorKind.AlreadySigned
        )
        expect(toSigningApiError(codedError("request-closed", {}, 409)).kind).toBe(
            SigningApiErrorKind.Closed
        )
    })

    it("reads a code left in the forwarded body", () => {
        const error = toSigningApiError(
            actionError(409, '{"message":"x","extensions":{"code":"already-signed"}}')
        )
        expect(error.kind).toBe(SigningApiErrorKind.AlreadySigned)
    })
})

const PANEL = {
    request: {
        id: "req-1",
        tenant_id: "tn-1",
        election_event_id: "ev-1",
        action: "close-voting",
        status: "waiting",
        code: "7F3A-91C2",
        canonical_payload: "{}",
        payload_sha256: "00",
        required: 2,
        expires_at: null,
        document_sha256: null,
    },
    rule: {requester_signing: "allowed"},
    count: 0,
    signers: [
        {user_id: "u-1", username: "maria", display_name: "Maria L. Santos", signed_at: null},
    ],
    document_url: null,
}

describe("validatePanel", () => {
    it.each([
        ["null", null],
        ["no request", {count: 0, signers: [], rule: {}}],
        ["signers not a list", {request: {}, rule: {}, count: 0, signers: {}}],
        [
            "a signer without display_name (PR 1's `name`)",
            {
                ...PANEL,
                signers: [{user_id: "u-1", username: "maria", name: "Maria", signed_at: null}],
            },
        ],
    ])("refuses %s", (_label, value) => {
        expect(() => validatePanel(value)).toThrow(SigningApiError)
    })
})

describe("createSigningApi", () => {
    const client = () => ({
        query: jest.fn(async () => ({data: {signingGetRequest: {panel: PANEL}}})),
        mutate: jest.fn(async ({mutation}: {mutation: DocumentNode; variables?: object}) => ({
            data: {[`signing${operationName(mutation)!.replace(/^Signing/, "")}`]: {ok: true}},
        })),
    })

    it("sends each route's input as the action's variables", async () => {
        const fake = client()
        const api = createSigningApi(fake as never)

        await expect(api.getRequest("req-1")).resolves.toEqual(PANEL)
        expect(fake.query).toHaveBeenCalledWith(
            expect.objectContaining({variables: {request_id: "req-1"}, fetchPolicy: "no-cache"})
        )

        await api.approve("req-1", {
            chain_pem: ["LEAF", "CA"],
            algorithm: SignatureAlgorithm.EcdsaP256Sha256,
            payload_signature_b64: "c2ln",
            pdf_cms_b64: "Y21z",
            revision: 3,
        })
        await api.reportOpenFailure("req-1", {
            file_name: "maria.p12",
            reason: CertificateOpenFailure.WrongPassword,
        })
        await api.cancel("req-1", {})
        const sent = fake.mutate.mock.calls.map(([{mutation, variables}]) => [
            operationName(mutation),
            variables,
        ])
        expect(sent).toEqual([
            [
                "SigningApprove",
                {
                    request_id: "req-1",
                    chain_pem: ["LEAF", "CA"],
                    algorithm: "ecdsa-p256-sha256",
                    payload_signature_b64: "c2ln",
                    pdf_cms_b64: "Y21z",
                    revision: 3,
                },
            ],
            [
                "SigningOpenFailure",
                {request_id: "req-1", file_name: "maria.p12", reason: "wrong-password"},
            ],
            ["SigningCancel", {request_id: "req-1", reason: null}],
        ])
    })

    it("refuses a malformed panel as a load failure", async () => {
        const fake = client()
        fake.query.mockResolvedValueOnce({
            data: {signingGetRequest: {panel: {count: 2} as unknown as typeof PANEL}},
        })
        await expect(createSigningApi(fake as never).getRequest("req-1")).rejects.toBeInstanceOf(
            SigningApiError
        )
    })

    it("turns a refused approval into a SigningApiError with its check", async () => {
        const fake = client()
        fake.mutate.mockRejectedValueOnce(actionError(422, '{"check":"registered-to-other"}'))
        const api = createSigningApi(fake as never)
        await expect(
            api.approve("req-1", {
                chain_pem: [],
                algorithm: SignatureAlgorithm.RsaPkcs1Sha256,
                payload_signature_b64: "",
            })
        ).rejects.toMatchObject({
            kind: SigningApiErrorKind.Refused,
            check: CertificateCheckId.RegisteredToOther,
        })
    })

    it("downloads a document's bytes and refuses a failed download", async () => {
        const bytes = new Uint8Array([60, 69, 77, 76, 62])
        const fetchOk = jest.fn(async () => new Response(bytes))
        await expect(
            createSigningApi(client() as never, fetchOk as never).fetchDocument("https://s3/eml")
        ).resolves.toEqual(bytes)
        expect(fetchOk).toHaveBeenCalledWith("https://s3/eml")

        const fetchDenied = jest.fn(async () => new Response("denied", {status: 403}))
        await expect(
            createSigningApi(client() as never, fetchDenied as never).fetchDocument(
                "https://s3/eml"
            )
        ).rejects.toMatchObject({status: 403})
    })
})

describe("createSigningApi roles", () => {
    /** An Apollo client whose link records each operation's `x-hasura-role` header. */
    const recordingClient = () => {
        const roles: Array<[string, string | undefined]> = []
        const client = new ApolloClient({
            cache: new InMemoryCache({addTypename: false}),
            link: new ApolloLink(
                (operation) =>
                    new Observable((observer) => {
                        roles.push([
                            operation.operationName,
                            operation.getContext().headers?.["x-hasura-role"],
                        ])
                        const field = `signing${operation.operationName.replace(/^Signing/, "")}`
                        observer.next({
                            data:
                                operation.operationName === "SigningGetRequest"
                                    ? {signingGetRequest: {panel: PANEL}}
                                    : {[field]: {request_id: "req-1"}},
                        })
                        observer.complete()
                    })
            ),
        })
        return {client, roles}
    }
    const holding =
        (...held: IPermissions[]) =>
        (permission: IPermissions) =>
            held.includes(permission)

    const run = async (api: ISigningApi) => {
        await api.getRequest("req-1")
        await api.checkCertificate("req-1", {chain_pem: ["LEAF"]})
        await api.approve("req-1", {
            chain_pem: ["LEAF"],
            algorithm: SignatureAlgorithm.EcdsaP256Sha256,
            payload_signature_b64: "c2ln",
        })
        await api.handover("req-1")
        await api.cancel("req-1", {})
    }

    it("sends a non-admin signer's own sign permission once the request names its action", async () => {
        const {client, roles} = recordingClient()
        await run(
            createSigningApi(client, undefined, {holds: holding(IPermissions.SIGN_CLOSE_VOTING)})
        )
        expect(roles).toEqual([
            ["SigningGetRequest", IPermissions.SIGN_CLOSE_VOTING],
            ["SigningCheckCertificate", IPermissions.SIGN_CLOSE_VOTING],
            ["SigningApprove", IPermissions.SIGN_CLOSE_VOTING],
            ["SigningHandover", IPermissions.SIGN_CLOSE_VOTING],
            // The requester cancels their own request with the action's sign permission.
            ["SigningCancel", IPermissions.SIGN_CLOSE_VOTING],
        ])
    })

    it("loads a request with signing-requests-read and cancels with signing-requests-cancel", async () => {
        const {client, roles} = recordingClient()
        const api = createSigningApi(client, undefined, {
            holds: holding(
                IPermissions.SIGNING_REQUESTS_READ,
                IPermissions.SIGNING_REQUESTS_CANCEL,
                IPermissions.SIGN_APPROVE_VOTER
            ),
        })
        await api.getRequest("req-1")
        await api.cancel("req-1", {})
        expect(roles).toEqual([
            ["SigningGetRequest", IPermissions.SIGNING_REQUESTS_READ],
            ["SigningCancel", IPermissions.SIGNING_REQUESTS_CANCEL],
        ])
    })

    it("sends admin-user for an admin without the action's sign permission", async () => {
        const {client, roles} = recordingClient()
        await run(createSigningApi(client, undefined, {holds: holding(IPermissions.ADMIN_USER)}))
        expect(roles.map(([, role]) => role)).toEqual(Array(5).fill(IPermissions.ADMIN_USER))
    })

    it("names no role without a permission check", async () => {
        const {client, roles} = recordingClient()
        await run(createSigningApi(client))
        expect(roles.map(([, role]) => role)).toEqual(Array(5).fill(undefined))
    })
})
