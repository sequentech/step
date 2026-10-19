// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {
    createContext,
    useCallback,
    useContext,
    useEffect,
    useMemo,
    useRef,
    useState,
} from "react"
import {ApolloError, useApolloClient} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {EXECUTE_PLUGIN, SET_PLUGIN_STATE, TENANT_PLUGINS} from "@/queries/PluginPlatform"
import {
    PluginErrorCode,
    PluginRequestError,
    PluginState,
    TenantPlugin,
    errorMessage,
} from "./contracts"

interface PluginContextValue {
    plugins: TenantPlugin[]
    loading: boolean
    error: string
    can: (permission: string) => boolean
    refresh: () => Promise<void>
    setState: (plugin: TenantPlugin, state: PluginState) => Promise<void>
    execute: <T>(id: string, operation: string, data: Record<string, unknown>) => Promise<T>
}

const unavailable = async (): Promise<never> => {
    throw new PluginRequestError("Plugin host is unavailable", PluginErrorCode.Unavailable)
}
const PluginContext = createContext<PluginContextValue>({
    plugins: [],
    loading: true,
    error: "",
    can: () => false,
    refresh: unavailable,
    setState: unavailable,
    execute: unavailable,
})
export const usePlugins = (): PluginContextValue => useContext(PluginContext)

const codes = new Set<string>(Object.values(PluginErrorCode))

/** Hasura forwards the gateway's message and extensions; no database details are exposed. */
export const pluginError = (error: unknown): PluginRequestError => {
    if (error instanceof PluginRequestError) return error
    const graph = error instanceof ApolloError ? error.graphQLErrors[0] : undefined
    const code = String(graph?.extensions?.code ?? "")
    return new PluginRequestError(
        graph?.message || errorMessage(error),
        codes.has(code) ? (code as PluginErrorCode) : PluginErrorCode.Unavailable,
        graph?.extensions?.current
    )
}

/**
 * Loads the tenant's plugins and runs their operations. Requests are scoped to
 * the authenticated tenant; a response that arrives after the tenant changed
 * is discarded.
 */
export const PluginProvider: React.FC<React.PropsWithChildren> = ({children}) => {
    const client = useApolloClient()
    const auth = useContext(AuthContext)
    const {t} = useTranslation()
    const [tenant] = useTenantStore()
    const scope = tenant && tenant === auth.tenantId && auth.isAuthenticated ? tenant : ""
    const currentScope = useRef(scope)
    currentScope.current = scope
    const [result, setResult] = useState<{scope: string; plugins: TenantPlugin[]}>({
        scope: "",
        plugins: [],
    })
    const [loading, setLoading] = useState(true)
    const [error, setError] = useState("")
    const {hasRole, updateTokenAndPermissionLabels} = auth

    const refresh = useCallback(async () => {
        if (!scope) {
            setLoading(false)
            return
        }
        setLoading(true)
        setError("")
        try {
            const response = await client.query<{
                tenant_plugins: {data: {plugins: TenantPlugin[]}}
            }>({
                query: TENANT_PLUGINS,
                fetchPolicy: "no-cache",
            })
            if (currentScope.current === scope) {
                setResult({scope, plugins: response.data.tenant_plugins.data.plugins})
            }
        } catch (cause) {
            if (currentScope.current === scope) setError(pluginError(cause).message)
        } finally {
            if (currentScope.current === scope) setLoading(false)
        }
    }, [client, scope])

    useEffect(() => {
        void refresh()
    }, [refresh])

    const execute = useCallback(
        async <T,>(id: string, operation: string, data: Record<string, unknown>): Promise<T> => {
            if (!scope) throw new PluginRequestError(t("plugins.signIn"), PluginErrorCode.Forbidden)
            let response
            try {
                response = await client.mutate<{execute_plugin_operation: {data: T}}>({
                    mutation: EXECUTE_PLUGIN,
                    variables: {plugin_id: id, operation, data},
                })
            } catch (cause) {
                throw pluginError(cause)
            }
            if (scope !== currentScope.current) {
                throw new PluginRequestError(t("plugins.signIn"), PluginErrorCode.Forbidden)
            }
            if (!response.data)
                throw new PluginRequestError("Empty plugin response", PluginErrorCode.Unavailable)
            return response.data.execute_plugin_operation.data
        },
        [client, scope, t]
    )

    const setState = useCallback(
        async (plugin: TenantPlugin, next: PluginState): Promise<void> => {
            if (!scope) throw new PluginRequestError(t("plugins.signIn"), PluginErrorCode.Forbidden)
            try {
                await client.mutate({
                    mutation: SET_PLUGIN_STATE,
                    variables: {
                        plugin_id: plugin.manifest.id,
                        state: next,
                        expected_version: plugin.version,
                    },
                })
            } catch (cause) {
                throw pluginError(cause)
            }
            // Turning a plugin on grants new roles; refresh the token to receive them.
            updateTokenAndPermissionLabels()
            await refresh()
        },
        [client, refresh, scope, t, updateTokenAndPermissionLabels]
    )

    const can = useCallback(
        (permission: string) => Boolean(scope) && hasRole(permission),
        [hasRole, scope]
    )

    const value = useMemo(
        () => ({
            plugins: result.scope === scope ? result.plugins : [],
            loading,
            error: scope ? error : t("plugins.signIn"),
            can,
            refresh,
            execute,
            setState,
        }),
        [can, error, execute, loading, refresh, result, scope, setState, t]
    )
    return <PluginContext.Provider value={value}>{children}</PluginContext.Provider>
}
