/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, renderHook, waitFor} from "@testing-library/react"
import {
    ApolloClient,
    ApolloLink,
    ApolloProvider,
    InMemoryCache,
    Observable,
    type MutationTuple,
} from "@apollo/client"
import {getOperationRole} from "@/services/Permissions"
import {IPermissions} from "@/types/keycloak"
import {
    useDeleteIssuer,
    useExportRequests,
    useImportIssuers,
    usePutChecks,
    usePutRule,
    useRegisterCertificate,
    useRevokeCertificate,
    useRuleCapacities,
    useSigningEventInfo,
    useWaitingSigningRequests,
    useWriteError,
    useWriteErrorMessage,
} from "./useSigningSettings"

// Keep the real shared predicate without loading the browser component barrel.
jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("@sequentech/ui-core"),
    ...jest.requireActual("../../../../../ui-core/src/utils/typechecks"),
}))
const mockNotify = jest.fn()
jest.mock("react-admin", () => ({useGetList: () => ({data: []}), useNotify: () => mockNotify}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("@/hooks/useAliasRenderer", () => ({useAliasRenderer: () => () => ""}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthorized: () => true}),
}))

/**
 * A client whose link picks the role as ApolloContextProvider does (an explicit
 * `x-hasura-role` wins over the operation's default) and records it per operation.
 */
const recordingClient = (isAdminUser: boolean) => {
    const roles: Record<string, string> = {}
    const client = new ApolloClient({
        cache: new InMemoryCache({addTypename: false}),
        link: new ApolloLink((operation) => {
            const explicit = operation.getContext().headers?.["x-hasura-role"]
            roles[operation.operationName] =
                explicit ?? getOperationRole(operation, false, isAdminUser)
            return new Observable((observer) => {
                observer.next({data: {}})
                observer.complete()
            })
        }),
    })
    const wrapper = ({children}: {children: React.ReactNode}) => (
        <ApolloProvider client={client}>{children}</ApolloProvider>
    )
    return {roles, wrapper}
}

/** A write hook of the tab; only its mutate function is called. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type WriteHook = () => MutationTuple<any, any>

const USERS = [
    ["an admin", true],
    ["staff without admin-user", false],
] as const

describe.each(USERS)("the Signatures tab's role for %s", (_label, isAdminUser) => {
    it("reads the rule capacities with signing-rules-read", async () => {
        const {roles, wrapper} = recordingClient(isAdminUser)
        renderHook(() => useRuleCapacities("event"), {wrapper})
        await waitFor(() =>
            expect(roles.GetSigningRuleCapacities).toBe(IPermissions.SIGNING_RULES_READ)
        )
    })

    it.each<[string, WriteHook, IPermissions]>([
        ["SigningPutRule", usePutRule, IPermissions.SIGNING_RULES_WRITE],
        ["SigningImportIssuers", useImportIssuers, IPermissions.SIGNING_ISSUERS_WRITE],
        ["SigningDeleteIssuer", useDeleteIssuer, IPermissions.SIGNING_ISSUERS_WRITE],
        ["SigningPutChecks", usePutChecks, IPermissions.SIGNING_CHECKS_WRITE],
        [
            "SigningRegisterCertificate",
            useRegisterCertificate,
            IPermissions.SIGNING_CERTIFICATES_REGISTER,
        ],
        [
            "SigningRevokeCertificate",
            useRevokeCertificate,
            IPermissions.SIGNING_CERTIFICATES_REVOKE,
        ],
        ["SigningExportRequests", useExportRequests, IPermissions.SIGNING_REQUESTS_EXPORT],
    ])("sends %s with its own permission", async (operationName, useWrite, permission) => {
        const {roles, wrapper} = recordingClient(isAdminUser)
        const {result} = renderHook(() => useWrite(), {wrapper})
        await act(async () => {
            await result.current[0]({variables: {}})
        })
        expect(roles[operationName]).toBe(permission)
    })
})

/** A client answering each role's rows, as Hasura filters them by action for `sign-<action>`. */
const roleClient = (rowsByRole: Record<string, Array<{id: string; created_at: string}>>) => {
    const sent: Array<{operation: string; role: string | undefined}> = []
    const client = new ApolloClient({
        cache: new InMemoryCache({addTypename: false}),
        link: new ApolloLink((operation) => {
            const role = operation.getContext().headers?.["x-hasura-role"]
            sent.push({operation: operation.operationName, role})
            return new Observable((observer) => {
                observer.next({
                    data: {
                        sequent_backend_signing_request: (rowsByRole[role] ?? []).map((row) => ({
                            action: "close-voting",
                            election_id: null,
                            area_id: null,
                            code: "7F3A-91C2",
                            required: 2,
                            expires_at: null,
                            approvals: [],
                            ...row,
                        })),
                    },
                })
                observer.complete()
            })
        }),
    })
    return {client, sent}
}

const holding = (client: ApolloClient<unknown>, held: IPermissions[]) => {
    const {AuthContext} = jest.requireMock("@/providers/AuthContextProvider")
    const isAuthorized = (_: boolean, __: string, permission: IPermissions) =>
        held.includes(permission)
    return ({children}: {children: React.ReactNode}) => (
        <ApolloProvider client={client}>
            <AuthContext.Provider value={{isAuthorized}}>{children}</AuthContext.Provider>
        </ApolloProvider>
    )
}

describe("the requests waiting for a signer", () => {
    it("are read once per sign permission held, each as its own Hasura role", async () => {
        const {client, sent} = roleClient({})
        const wrapper = holding(client, [
            IPermissions.SIGN_CLOSE_VOTING,
            IPermissions.SIGN_APPROVE_VOTER,
            // A trustee's requests aren't in Hasura; reading and admin roles don't sign.
            IPermissions.SIGN_KEY_CEREMONY,
            IPermissions.SIGN_TALLY_KEY,
            IPermissions.SIGNING_REQUESTS_READ,
            IPermissions.ADMIN_USER,
        ])
        const {result} = renderHook(() => useWaitingSigningRequests("event"), {wrapper})
        await waitFor(() => expect(result.current.requests).toEqual([]))
        expect(sent.map(({role}) => role).sort()).toEqual(
            [IPermissions.SIGN_APPROVE_VOTER, IPermissions.SIGN_CLOSE_VOTING].sort()
        )
        expect(new Set(sent.map(({operation}) => operation))).toEqual(
            new Set(["GetWaitingSigningRequests"])
        )
    })

    it("lists each role's requests once, oldest first", async () => {
        const {client} = roleClient({
            [IPermissions.SIGN_CLOSE_VOTING]: [
                {id: "b", created_at: "2028-05-08T11:00:00Z"},
                {id: "a", created_at: "2028-05-08T10:00:00Z"},
            ],
            [IPermissions.SIGN_OPEN_VOTING]: [{id: "c", created_at: "2028-05-08T10:30:00Z"}],
        })
        const wrapper = holding(client, [
            IPermissions.SIGN_CLOSE_VOTING,
            IPermissions.SIGN_OPEN_VOTING,
        ])
        const {result} = renderHook(() => useWaitingSigningRequests("event"), {wrapper})
        await waitFor(() =>
            expect(result.current.requests?.map(({id}) => id)).toEqual(["a", "c", "b"])
        )
    })

    it("reads nothing without a sign permission", async () => {
        const {client, sent} = roleClient({})
        const wrapper = holding(client, [IPermissions.SIGNING_REQUESTS_READ])
        const {result} = renderHook(() => useWaitingSigningRequests("event"), {wrapper})
        await waitFor(() => expect(result.current.requests).toEqual([]))
        expect(sent).toEqual([])
        expect(result.current.roles).toEqual([])
    })
})

describe("the event's time zone and the signers' titles", () => {
    it.each<[string, IPermissions[], IPermissions | undefined]>([
        [
            "a certificates reader, who also gets the titles",
            [IPermissions.SIGNING_REQUESTS_READ, IPermissions.SIGNING_CERTIFICATES_READ],
            IPermissions.SIGNING_CERTIFICATES_READ,
        ],
        [
            "a requests reader",
            [IPermissions.SIGNING_REQUESTS_READ],
            IPermissions.SIGNING_REQUESTS_READ,
        ],
        ["a rules reader", [IPermissions.SIGNING_RULES_READ], IPermissions.SIGNING_RULES_READ],
        ["a signer", [IPermissions.SIGN_APPROVE_VOTER], IPermissions.SIGN_APPROVE_VOTER],
    ])("are read by %s with a role it holds", async (_label, held, role) => {
        const {client, sent} = roleClient({})
        renderHook(() => useSigningEventInfo("event"), {wrapper: holding(client, held)})
        await waitFor(() => expect(sent).toEqual([{operation: "SigningEventInfo", role}]))
    })

    it("are not read by someone holding none of those", async () => {
        const {client, sent} = roleClient({})
        const {result} = renderHook(() => useSigningEventInfo("event"), {
            wrapper: holding(client, [IPermissions.ADMIN_USER]),
        })
        await waitFor(() => expect(result.current.timeZone).toBeNull())
        expect(sent).toEqual([])
    })
})

describe("signing write explanations", () => {
    beforeEach(() => mockNotify.mockClear())

    it("formats the trustee explanation for an inline alert without sending a notification", () => {
        const {result} = renderHook(() => useWriteErrorMessage())
        expect(
            result.current(
                {
                    graphQLErrors: [
                        {
                            message: "Trustees cannot sign automatic steps.",
                            extensions: {code: "invalid", reason: "automated-ceremonies"},
                        },
                    ],
                },
                "signing.rule.saveError"
            )
        ).toBe("signing.errors.automatedCeremonies")
        expect(mockNotify).not.toHaveBeenCalled()
    })

    it("returns and notifies the specific automatic-ceremony explanation", () => {
        const {result} = renderHook(() => useWriteError())
        let message: string | undefined
        act(() => {
            message = result.current(
                {
                    graphQLErrors: [
                        {
                            message: "Trustees cannot sign automatic steps.",
                            extensions: {code: "invalid", reason: "automated-ceremonies"},
                        },
                    ],
                },
                "signing.rule.saveError"
            )
        })
        expect(message).toBe("signing.errors.automatedCeremonies")
        expect(mockNotify).toHaveBeenCalledWith(message, {type: "error"})
    })

    it("uses a localized validation message instead of the server error body", () => {
        const {result} = renderHook(() => useWriteError())
        const message = "The expiry must be at least one minute."
        expect(
            result.current(
                {graphQLErrors: [{message, extensions: {code: "invalid"}}]},
                "signing.rule.saveError"
            )
        ).toBe("signing.errors.invalid")
        expect(mockNotify).toHaveBeenCalledWith("signing.errors.invalid", {type: "error"})
    })

    it("keeps a translated fallback for unexpected or missing server detail", () => {
        const {result} = renderHook(() => useWriteError())
        expect(
            result.current(new Error("Internal network stack detail"), "signing.rule.saveError")
        ).toBe("signing.rule.saveError")
        expect(
            result.current(
                {graphQLErrors: [{message: "  ", extensions: {code: "invalid"}}]},
                "signing.rule.saveError"
            )
        ).toBe("signing.errors.invalid")
    })
})
