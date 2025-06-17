// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// ====================================================================================
// 1. IMPORTS - All dependencies for the module in one place
// ====================================================================================

import {useState, useEffect, useCallback, createContext, useContext} from "react"
import initSqlJs, {Database} from "sql.js"
import {openDB} from "idb"
import {useQuery} from "@apollo/client"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument" // Adjust path if needed
import {FetchDocumentQuery} from "@/gql/graphql" // Adjust path if needed

// ====================================================================================
// 2. TYPE DEFINITIONS & CONSTANTS
// ====================================================================================

interface QueryResult {
    [key: string]: any
}

const IDB_NAME = "sqlite-databases"
const IDB_STORE_NAME = "database-cache"

// ====================================================================================
// 3. INTERNAL HOOKS - Helpers not exported from the module
// ====================================================================================

/**
 * Initializes the IndexedDB database.
 */
const initIdb = () => {
    return openDB(IDB_NAME, 1, {
        upgrade(db) {
            db.createObjectStore(IDB_STORE_NAME)
        },
    })
}

/**
 * An internal hook to download a database from a remote URL (via Hasura)
 * and cache it in IndexedDB. It is not exported.
 * @param electionEventId The Id of the election event to recover the database from.
 * @param documentId The document Id of the database file.
 */
function _useCachedDatabase(
    electionEventId?: string,
    documentId?: string,
    enabled: boolean = true
) {
    // DO NOT return early. All hooks must be called on every render.
    const [dbData, setDbData] = useState<Uint8Array | null>(null)
    const [isLoading, setIsLoading] = useState(false)
    const [error, setError] = useState<string | null>(null)

    // ✅ The 'enabled' flag is passed to the 'skip' option.
    // This is the correct way to conditionally run a query.
    const {
        loading: isLoadingUrl,
        error: urlError,
        data: documentData,
    } = useQuery<FetchDocumentQuery>(FETCH_DOCUMENT, {
        variables: {electionEventId, documentId},
        skip: !enabled || !electionEventId || !documentId,
    })

    const remoteUrl = documentData?.fetchDocument?.url

    useEffect(() => {
        // ✅ The condition is now INSIDE the effect. This is allowed.
        // The useEffect hook itself is always called, but its logic only runs when enabled.
        if (!enabled || !documentId || !remoteUrl) {
            // If we are not enabled, ensure data is cleared for subsequent runs.
            if (!enabled) setDbData(null)
            return
        }

        const loadDatabase = async () => {
            setError(null)
            try {
                const db = await initIdb()
                const cachedData = await db.get(IDB_STORE_NAME, documentId)

                if (cachedData) {
                    setDbData(cachedData)
                } else {
                    setIsLoading(true)
                    const response = await fetch(remoteUrl)

                    if (!response.ok) {
                        throw new Error(
                            `Failed to fetch database from URL: ${response.statusText} (status: ${response.status})`
                        )
                    }

                    const arrayBuffer = await response.arrayBuffer()
                    const data = new Uint8Array(arrayBuffer)

                    await db.put(IDB_STORE_NAME, data, documentId)
                    setDbData(data)
                }
            } catch (err: any) {
                console.error("[_useCachedDatabase] CRITICAL ERROR:", err)
                setError(err.message)
            } finally {
                setIsLoading(false)
            }
        }

        loadDatabase()
        // Add `enabled` to the dependency array
    }, [enabled, documentId, remoteUrl])

    return {
        dbData,
        // The isLoading/error states should only be active if the hook is enabled
        isLoading: enabled && (isLoadingUrl || isLoading),
        error: enabled ? (urlError ? urlError.message : error) : null,
    }
}

// ====================================================================================
// 4. PUBLIC CONTEXT - For sharing DB instances across the app
// ====================================================================================

interface DatabaseContextType {
    databases: Map<string, Database>
    addDatabase: (name: string, database: Database) => void
    removeDatabase: (name: string) => void
}

const DatabaseContext = createContext<DatabaseContextType | null>(null)

export function useDatabaseContext() {
    const context = useContext(DatabaseContext)
    if (!context) throw new Error("useDatabaseContext must be used within a DatabaseProvider")
    return context
}

export function useDatabaseManager() {
    const [databases, setDatabases] = useState(new Map<string, Database>())

    const addDatabase = useCallback((name: string, database: Database) => {
        setDatabases((prev) => new Map(prev).set(name, database))
    }, [])

    const removeDatabase = useCallback((name: string) => {
        setDatabases((prev) => {
            const newMap = new Map(prev)
            newMap.delete(name)
            return newMap
        })
    }, [])

    const contextValue = {databases, addDatabase, removeDatabase}

    return {...contextValue, contextValue}
}

export {DatabaseContext}

// ====================================================================================
// 5. PUBLIC HOOKS - The primary API for components
// ====================================================================================

/**
 * Manages loading a database and placing it into the global context.
 * This is the primary hook components should use to ensure a database is available.
 */
export function useManagedDatabase(documentId?: string, electionEventId?: string) {
    const {databases, addDatabase} = useDatabaseContext()

    // 1. First, check if the fully initialized DB is already in our React context (memory).
    const isAlreadyInContext = documentId ? databases.has(documentId) : false

    // 2. We only need to run the more expensive `_useCachedDatabase` hook if it's NOT in the context.
    const shouldLoadDatabase = !!documentId && !isAlreadyInContext

    // 3. Call `_useCachedDatabase` and pass our conditional 'enabled' flag.
    //    If shouldLoadDatabase is false, this hook will do nothing and return immediately.
    const {
        dbData,
        isLoading: isCacheLoading,
        error: cacheError,
    } = _useCachedDatabase(electionEventId, documentId, shouldLoadDatabase)

    const [isInitializing, setIsInitializing] = useState(false)
    const [initError, setInitError] = useState<string | null>(null)

    // This useEffect for initializing the sql.js instance remains the same.
    // It will only run if `shouldLoadDatabase` was true and `dbData` was successfully retrieved.
    useEffect(() => {
        if (!dbData || !documentId || databases.has(documentId)) {
            return
        }

        const initialize = async () => {
            setIsInitializing(true)
            setInitError(null)
            try {
                const sql = await initSqlJs({
                    locateFile: (file) => `https://sql.js.org/dist/${file}`,
                })
                const db = new sql.Database(dbData)
                addDatabase(documentId, db)
            } catch (e) {
                const errorMessage = e instanceof Error ? e.message : "Unknown initialization error"
                setInitError(errorMessage)
            } finally {
                setIsInitializing(false)
            }
        }
        initialize()
    }, [documentId, dbData, databases, addDatabase])

    // The final "ready" state is always derived directly from the context.
    const isReady = isAlreadyInContext

    return {
        // We are "loading" only if we determined that we should be loading.
        isLoading: shouldLoadDatabase && (isCacheLoading || isInitializing),
        error: cacheError || initError,
        isReady,
        databaseName: documentId,
    }
}

/**
 * Executes a SQL query against a database managed in the context.
 */
export function useSQLQuery<T = QueryResult>(
    sql: string,
    params: any[] = [],
    options: {enabled?: boolean; databaseName?: string} = {}
) {
    const {enabled = true, databaseName} = options
    const {databases} = useDatabaseContext() // Get the map of all DBs

    const [data, setData] = useState<T[]>([])
    const [isLoading, setIsLoading] = useState(false)
    const [error, setError] = useState<string | null>(null)

    const executeQuery = useCallback(async () => {
        // We now check for the databaseName and enabled status right at the start.
        if (!enabled || !databaseName) {
            return
        }

        // We get the specific database instance from the map inside the callback.
        const db = databases.get(databaseName)

        // If the database isn't loaded into the context yet, we just wait.
        if (!db) {
            setIsLoading(true) // It's loading, but the DB isn't ready yet.
            return
        }

        setIsLoading(true)
        setError(null)
        try {
            const stmt = db.prepare(sql, params)
            const results: T[] = []
            while (stmt.step()) {
                results.push(stmt.getAsObject() as T)
            }
            stmt.free()
            setData(results)
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : "Unknown query error"
            setError(errorMessage)
        } finally {
            setIsLoading(false)
        }
        // By adding `databaseName` and `databases` here, this function is guaranteed
        // to be re-created and re-run when the name changes or when the DB is finally loaded.
    }, [databaseName, databases, sql, JSON.stringify(params), enabled])

    useEffect(() => {
        executeQuery()
    }, [executeQuery])

    return {data, isLoading, error, refetch: executeQuery}
}
