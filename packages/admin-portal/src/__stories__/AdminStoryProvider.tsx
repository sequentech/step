// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState, type PropsWithChildren} from "react"
import {ApolloClient, ApolloLink, ApolloProvider, InMemoryCache, Observable} from "@apollo/client"
import type {FetchResult, Operation} from "@apollo/client"
import {AdminContext, Notification, testDataProvider, type DataProvider} from "react-admin"
import {QueryClient} from "@tanstack/react-query"
import {createStore, Provider as AtomProvider} from "jotai"
import {fullAdminTheme} from "@/services/AdminTheme"
import {adminI18nProvider} from "@/services/AdminTranslation"
import {TenantContext} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import cssInputLookAndFeel from "@/atoms/css-input-look-and-feel"
import {StyledAppAtom} from "@/components/StyledAppAtom"
import {
    STORY_BRANDING,
    type EStoryPermissions,
    type EStoryTenant,
} from "../../../ui-essentials/.storybook/globals"
import {storyAuth} from "./storyAuth"

export const TENANT_ID = "11111111-1111-4111-8111-111111111111"
export const EVENT_ID = "22222222-2222-4222-8222-222222222222"

export interface RecordedOperation {
    name: string
    variables: Record<string, unknown>
    headers: Record<string, unknown>
}

/** A handler's promise answers when it settles; one that never settles keeps the query loading. */
export function graphqlBoundary(
    handlers: Record<string, (operation: Operation) => FetchResult | Promise<FetchResult>>
) {
    const calls: RecordedOperation[] = []
    const unexpected: string[] = []
    const client = new ApolloClient({
        cache: new InMemoryCache(),
        link: new ApolloLink(
            (operation) =>
                new Observable((observer) => {
                    calls.push({
                        name: operation.operationName,
                        variables: operation.variables,
                        headers: operation.getContext().headers ?? {},
                    })
                    const handler = handlers[operation.operationName]
                    if (!handler) {
                        unexpected.push(operation.operationName)
                        observer.error(
                            new Error(`Unexpected operation: ${operation.operationName}`)
                        )
                        return
                    }
                    try {
                        const result = handler(operation)
                        if (result instanceof Promise) {
                            result.then(
                                (value) => {
                                    observer.next(value)
                                    observer.complete()
                                },
                                (error: unknown) => observer.error(error)
                            )
                            return
                        }
                        observer.next(result)
                        observer.complete()
                    } catch (error) {
                        observer.error(error)
                    }
                })
        ),
    })
    return {client, calls, unexpected}
}

/** Applies the tenant's look-and-feel CSS as the admin application does. */
function TenantLookAndFeel({tenant, children}: PropsWithChildren<{tenant: EStoryTenant}>) {
    const css = STORY_BRANDING[tenant].css ?? ""
    const [store] = useState(() => {
        const atoms = createStore()
        atoms.set(cssInputLookAndFeel, css)
        return atoms
    })
    useEffect(() => {
        store.set(cssInputLookAndFeel, css)
    }, [store, css])
    return (
        <AtomProvider store={store}>
            <StyledAppAtom>{children}</StyledAppAtom>
        </AtomProvider>
    )
}

export function AdminStoryProvider({
    children,
    boundary,
    dataProvider = testDataProvider(),
    role,
    tenant,
}: PropsWithChildren<{
    boundary: ReturnType<typeof graphqlBoundary>
    dataProvider?: DataProvider
    /** Signs in a member of this role group, typically the story's `permissions` global. */
    role?: EStoryPermissions
    /** Applies this tenant's branding, typically the story's `tenant` global. */
    tenant?: EStoryTenant
}>) {
    const auth = useContext(AuthContext)
    const [queryClient] = React.useState(
        () =>
            new QueryClient({defaultOptions: {queries: {retry: false}, mutations: {retry: false}}})
    )
    const signedIn = role ? (
        <AuthContext.Provider value={storyAuth(role, TENANT_ID, auth)}>
            {children}
        </AuthContext.Provider>
    ) : (
        children
    )
    return (
        <AdminContext
            dataProvider={dataProvider}
            queryClient={queryClient}
            theme={fullAdminTheme}
            i18nProvider={adminI18nProvider}
        >
            <ApolloProvider client={boundary.client}>
                <TenantContext.Provider
                    value={{tenantId: TENANT_ID, setTenantId: () => {}, setTenant: () => {}}}
                >
                    {tenant ? (
                        <TenantLookAndFeel tenant={tenant}>{signedIn}</TenantLookAndFeel>
                    ) : (
                        signedIn
                    )}
                    <Notification />
                </TenantContext.Provider>
            </ApolloProvider>
        </AdminContext>
    )
}
