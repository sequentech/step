// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useState, useEffect} from "react"
import {openDB} from "idb"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"
import {FetchDocumentQuery} from "@/gql/graphql"
import {useQuery} from "@apollo/client"

const DB_NAME = "sqlite-databases"
const STORE_NAME = "database-cache"

// Helper function to initialize the IndexedDB database
const initIdb = () => {
    return openDB(DB_NAME, 1, {
        upgrade(db) {
            db.createObjectStore(STORE_NAME)
        },
    })
}

/**
 * A hook to download a database from a remote URL and cache it in IndexedDB.
 * @param electionEventId The Id of the election event to recover the database from.
 * @param documentId The document Id of the database file.
 * @returns An object with the database data as a Uint8Array, loading state, and error state.
 */
export const useCachedDatabase = (
    electionEventId: string | undefined,
    documentId: string | undefined
) => {
    const [dbData, setDbData] = useState<Uint8Array | null>(null)
    // This state now tracks the caching/downloading process specifically
    const [isCaching, setIsCaching] = useState(false)
    const [error, setError] = useState<string | null>(null)

    // --- 1. Fetch the document URL at the TOP LEVEL of the hook ---
    const {
        loading: isLoadingUrl,
        error: urlError,
        data: documentData,
    } = useQuery<FetchDocumentQuery>(FETCH_DOCUMENT, {
        variables: {
            electionEventId,
            documentId,
        },
        // Use the 'skip' option to prevent the query from running if IDs are missing
        skip: !electionEventId || !documentId,
    })

    // Extract the final URL once the query is complete
    const remoteUrl = documentData?.fetchDocument?.url

    // --- 2. Use a SEPARATE useEffect to handle the caching logic ---
    // This effect runs whenever the documentId or the fetched remoteUrl changes.
    useEffect(() => {
        // Don't do anything if we don't have the necessary IDs
        if (!documentId || !remoteUrl) {
            return
        }

        const loadDatabase = async () => {
            setError(null)

            try {
                const db = await initIdb()
                // 1. Try to get the database from the cache first
                const cachedData = await db.get(STORE_NAME, documentId)

                if (cachedData) {
                    console.log("Database loaded from IndexedDB cache.")
                    setDbData(cachedData)
                } else {
                    // 2. If not in cache, fetch from the now-known remoteUrl
                    setIsCaching(true) // Start caching loading state
                    console.log("Fetching database from remote URL:", remoteUrl)
                    const response = await fetch(remoteUrl)
                    if (!response.ok) {
                        throw new Error(`Failed to fetch database: ${response.statusText}`)
                    }
                    const arrayBuffer = await response.arrayBuffer()
                    const data = new Uint8Array(arrayBuffer)

                    // 3. Store the new database in the cache
                    await db.put(STORE_NAME, data, documentId)
                    console.log("Database cached in IndexedDB.")
                    setDbData(data)
                }
            } catch (err: any) {
                setError(err.message)
                console.error("Error loading/caching database:", err)
            } finally {
                setIsCaching(false) // Stop caching loading state
            }
        }

        loadDatabase()
    }, [documentId, remoteUrl]) // Dependency array ensures this runs when we get a URL

    return {
        dbData,
        // The overall loading state is true if we are fetching the URL OR caching the file
        isLoading: isLoadingUrl || isCaching,
        // The overall error is the URL fetch error OR the caching error
        error: urlError ? urlError.message : error,
    }
}
