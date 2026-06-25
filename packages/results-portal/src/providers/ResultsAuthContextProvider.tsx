// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {createContext, useCallback, useContext, useMemo, useState} from "react"

export interface ResultsUserProfile {
    firstName?: string
    username: string
    email?: string
    openLink?: () => void
}

export interface ResultsAuthenticatedSession {
    userProfile: ResultsUserProfile
    logout: () => void
}

interface ResultsAuthContextValue {
    isAuthenticated: boolean
    userProfile?: ResultsUserProfile
    logout?: () => void
    setAuthenticatedSession: (session: ResultsAuthenticatedSession) => void
    clearAuthenticatedSession: () => void
}

const ResultsAuthContext = createContext<ResultsAuthContextValue>({
    isAuthenticated: false,
    setAuthenticatedSession: () => undefined,
    clearAuthenticatedSession: () => undefined,
})

export const ResultsAuthContextProvider: React.FC<{children: React.ReactNode}> = ({children}) => {
    const [session, setSession] = useState<ResultsAuthenticatedSession | undefined>(undefined)
    const setAuthenticatedSession = useCallback(
        (nextSession: ResultsAuthenticatedSession) => setSession(nextSession),
        []
    )
    const clearAuthenticatedSession = useCallback(() => setSession(undefined), [])
    const value = useMemo<ResultsAuthContextValue>(
        () => ({
            isAuthenticated: Boolean(session),
            userProfile: session?.userProfile,
            logout: session?.logout,
            setAuthenticatedSession,
            clearAuthenticatedSession,
        }),
        [clearAuthenticatedSession, session, setAuthenticatedSession]
    )

    return <ResultsAuthContext.Provider value={value}>{children}</ResultsAuthContext.Provider>
}

export const useResultsAuth = (): ResultsAuthContextValue => useContext(ResultsAuthContext)
