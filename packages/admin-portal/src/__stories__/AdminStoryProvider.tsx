// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {type PropsWithChildren} from "react"
import {ApolloClient, ApolloLink, ApolloProvider, InMemoryCache, Observable} from "@apollo/client"
import type {FetchResult, Operation} from "@apollo/client"
import {AdminContext, Notification, testDataProvider, type DataProvider} from "react-admin"
import {QueryClient} from "@tanstack/react-query"
import {fullAdminTheme} from "@/services/AdminTheme"
import {adminI18nProvider} from "@/services/AdminTranslation"
import {TenantContext} from "@/providers/TenantContextProvider"

export const TENANT_ID = "11111111-1111-4111-8111-111111111111"
export const EVENT_ID = "22222222-2222-4222-8222-222222222222"

export interface RecordedOperation {
    name: string
    variables: Record<string, unknown>
    headers: Record<string, unknown>
}

export function graphqlBoundary(handlers: Record<string, (operation: Operation) => FetchResult>) {
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
                        observer.next(handler(operation))
                        observer.complete()
                    } catch (error) {
                        observer.error(error)
                    }
                })
        ),
    })
    return {client, calls, unexpected}
}

export function AdminStoryProvider({
    children,
    boundary,
    dataProvider = testDataProvider(),
}: PropsWithChildren<{
    boundary: ReturnType<typeof graphqlBoundary>
    dataProvider?: DataProvider
}>) {
    const [queryClient] = React.useState(
        () =>
            new QueryClient({defaultOptions: {queries: {retry: false}, mutations: {retry: false}}})
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
                    {children}
                    <Notification />
                </TenantContext.Provider>
            </ApolloProvider>
        </AdminContext>
    )
}
