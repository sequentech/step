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
