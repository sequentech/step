// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState, type PropsWithChildren} from "react"
import {ApolloClient, ApolloLink, ApolloProvider, InMemoryCache, Observable} from "@apollo/client"
import type {FetchResult, Operation} from "@apollo/client"
import {
    AdminContext,
    Notification,
    memoryStore,
    testDataProvider,
    type DataProvider,
    type Store,
} from "react-admin"
import {QueryClient} from "@tanstack/react-query"
import {createStore, Provider as AtomProvider} from "jotai"
import {
    buildClientSchema,
    defaultFieldResolver,
    execute,
    validate,
    type GraphQLFieldResolver,
    type GraphQLSchema,
    type IntrospectionQuery,
} from "graphql"
import {fullAdminTheme} from "@/services/AdminTheme"
import {adminI18nProvider} from "@/services/AdminTranslation"
import {TenantContext} from "@/providers/TenantContextProvider"
import {AuthContext, type AuthContextValues} from "@/providers/AuthContextProvider"
import {SettingsContext, type GlobalSettings} from "@/providers/SettingsContextProvider"
import type {Sequent_Backend_Tenant} from "@/gql/graphql"
import cssInputLookAndFeel from "@/atoms/css-input-look-and-feel"
import {StyledAppAtom} from "@/components/StyledAppAtom"
import {
    STORY_BRANDING,
    type EStoryPermissions,
    type EStoryTenant,
} from "../../../ui-essentials/.storybook/globals"
import {storyAuth} from "./storyAuth"
import {registerBoundary} from "./storyNetwork"

export const TENANT_ID = "11111111-1111-4111-8111-111111111111"
export const EVENT_ID = "22222222-2222-4222-8222-222222222222"

export interface RecordedOperation {
    name: string
    variables: Record<string, unknown>
    headers: Record<string, unknown>
}

let adminSchema: Promise<GraphQLSchema> | undefined
/** The portal's GraphQL schema, from the introspection file the code generator uses. */
const loadAdminSchema = () =>
    (adminSchema ??= import("virtual:admin-graphql-schema").then(({default: text}) => {
        const introspection = JSON.parse(text) as IntrospectionQuery | {data: IntrospectionQuery}
        return buildClientSchema("data" in introspection ? introspection.data : introspection)
    }))

// Mock data may use the response key (an alias) or the field name.
const aliasOrField: GraphQLFieldResolver<unknown, unknown> = (source, args, context, info) => {
    const record = source as Record<string, unknown> | null
    if (record && typeof record === "object" && info.path.key in record) {
        const value = record[info.path.key as string]
        return typeof value === "function" ? value(args, context, info) : value
    }
    return defaultFieldResolver(source, args, context, info)
}

export interface GraphqlBoundaryOptions {
    /**
     * Validates each operation and its variables against the admin schema and
     * executes the handler's `data` against it: the reply is type checked,
     * trimmed to the selection set and given its `__typename` fields. A
     * mismatch is recorded in `unexpected`.
     */
    schema?: boolean
}

/** A handler's promise answers when it settles; one that never settles keeps the query loading. */
export function graphqlBoundary(
    handlers: Record<string, (operation: Operation) => FetchResult | Promise<FetchResult>>,
    {schema = false}: GraphqlBoundaryOptions = {}
) {
    const calls: RecordedOperation[] = []
    const unexpected: string[] = []
    const answer = async (operation: Operation): Promise<FetchResult> => {
        const handler = handlers[operation.operationName]
        if (!schema) return handler(operation)
        const types = await loadAdminSchema()
        const problems = validate(types, operation.query).map(({message}) => message)
        if (problems.length) {
            unexpected.push(`Invalid operation ${operation.operationName}: ${problems.join("; ")}`)
            throw new Error(`Invalid operation ${operation.operationName}`)
        }
        const reply = await handler(operation)
        if (!reply.data || reply.errors?.length) return reply
        const result = await execute({
            schema: types,
            document: operation.query,
            operationName: operation.operationName,
            variableValues: operation.variables,
            rootValue: reply.data,
            fieldResolver: aliasOrField,
        })
        if (result.errors?.length) {
            unexpected.push(
                `Mock data for ${operation.operationName} does not match the schema: ` +
                    result.errors.map(({message}) => message).join("; ")
            )
        }
        return {...reply, data: result.data ?? null}
    }
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
                    if (!handlers[operation.operationName]) {
                        unexpected.push(operation.operationName)
                        observer.error(
                            new Error(`Unexpected operation: ${operation.operationName}`)
                        )
                        return
                    }
                    try {
                        const result = schema
                            ? answer(operation)
                            : handlers[operation.operationName](operation)
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
    // Loading the schema takes a moment; a story awaits `ready` in its beforeEach
    // so that the first reply is not slower than its assertions' timeout.
    const ready = schema ? loadAdminSchema().then(() => undefined) : Promise.resolve()
    return registerBoundary({client, calls, unexpected, ready})
}

/**
 * Settings of a story: nothing polls, and every service address is on the
 * reserved `.invalid` domain, so a request that escaped the boundaries could
 * never reach a developer's local stack.
 */
export const STORY_SETTINGS: Partial<GlobalSettings> = {
    QUERY_POLL_INTERVAL_MS: 3_600_000,
    QUERY_FAST_POLL_INTERVAL_MS: 3_600_000,
    DEFAULT_TENANT_ID: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
    KEYCLOAK_URL: "https://keycloak.admin-story.invalid/",
    HASURA_URL: "https://hasura.admin-story.invalid/v1/graphql",
    PUBLIC_BUCKET_URL: "https://public.admin-story.invalid/",
    VOTING_PORTAL_URL: "https://voting.admin-story.invalid",
    KIOSK_VOTING_PORTAL_URL: "https://kiosk.admin-story.invalid",
    RESULTS_PORTAL_URL: "https://results.admin-story.invalid",
    IVR_EMULATOR_BASE_URL: "https://ivr.admin-story.invalid",
    CUSTOM_URLS_DOMAIN_NAME: "admin-story.invalid",
    APP_VERSION: "story",
    APP_HASH: "story",
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
    roles,
    auth: authOverrides,
    tenant,
    tenantRecord,
    settings,
    store,
}: PropsWithChildren<{
    boundary: ReturnType<typeof graphqlBoundary>
    dataProvider?: DataProvider
    /** Signs in a member of this role group, typically the story's `permissions` global. */
    role?: EStoryPermissions
    /** Replaces the group's roles, for a story about one permission. */
    roles?: string[]
    /** Further signed-in user values, e.g. `isGoldUser` or `permissionLabels`. */
    auth?: Partial<AuthContextValues>
    /** Applies this tenant's branding, typically the story's `tenant` global. */
    tenant?: EStoryTenant
    /** The selected tenant's record, as TenantContextProvider stores it. */
    tenantRecord?: Sequent_Backend_Tenant
    /** Replaces story settings; the defaults keep polling and service addresses offline. */
    settings?: Partial<GlobalSettings>
    /** React-admin's preference store; each story starts from an empty memory store. */
    store?: Store
}>) {
    const auth = useContext(AuthContext)
    const baseSettings = useContext(SettingsContext)
    const [queryClient] = React.useState(
        () =>
            new QueryClient({defaultOptions: {queries: {retry: false}, mutations: {retry: false}}})
    )
    const [preferences] = useState(() => store ?? memoryStore())
    const signedIn = role ? storyAuth(role, TENANT_ID, auth) : auth
    const user: AuthContextValues = {
        ...signedIn,
        ...(roles
            ? {
                  hasRole: (name: string) => roles.includes(name),
                  isAuthorized: (_checkSuperAdmin, someTenantId, permission) =>
                      someTenantId === TENANT_ID &&
                      [permission].flat().some((name) => roles.includes(name)),
              }
            : {}),
        ...authOverrides,
    }
    const content =
        role || roles || authOverrides ? (
            <AuthContext.Provider value={user}>{children}</AuthContext.Provider>
        ) : (
            children
        )
    return (
        <AdminContext
            dataProvider={dataProvider}
            queryClient={queryClient}
            store={preferences}
            theme={fullAdminTheme}
            i18nProvider={adminI18nProvider}
        >
            <ApolloProvider client={boundary.client}>
                <SettingsContext.Provider
                    value={{
                        loaded: true,
                        globalSettings: {
                            ...baseSettings.globalSettings,
                            ...STORY_SETTINGS,
                            ...settings,
                        },
                    }}
                >
                    <TenantContext.Provider
                        value={{
                            tenantId: TENANT_ID,
                            tenant: tenantRecord,
                            setTenantId: () => {},
                            setTenant: () => {},
                        }}
                    >
                        {tenant ? (
                            <TenantLookAndFeel tenant={tenant}>{content}</TenantLookAndFeel>
                        ) : (
                            content
                        )}
                        <Notification />
                    </TenantContext.Provider>
                </SettingsContext.Provider>
            </ApolloProvider>
        </AdminContext>
    )
}
