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
} from "./useSigningSettings"

// Keep the real shared predicate without loading the browser component barrel.
jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("@sequentech/ui-core"),
    ...jest.requireActual("../../../../../ui-core/src/utils/typechecks"),
}))
jest.mock("react-admin", () => ({useGetList: () => ({data: []}), useNotify: () => jest.fn()}))
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
