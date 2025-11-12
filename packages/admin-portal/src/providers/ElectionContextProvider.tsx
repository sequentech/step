// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext, useState} from "react"

interface ElectionContextProps {
    ElectionId: string | null
    setElectionId: (ElectionId: string | null) => void
}

const defaultElectionContext: ElectionContextProps = {
    ElectionId: null,
    setElectionId: () => undefined,
}

export const ElectionContext = createContext<ElectionContextProps>(defaultElectionContext)

interface ElectionContextProviderProps {
    /**
     * The elements wrapped by the Election context.
     */
    children: React.ReactNode
}

export const ElectionContextProvider = (props: ElectionContextProviderProps) => {
    const [Election, setElection] = useState<string | null>(
        localStorage.getItem("selected-election-event-id") || null
    )

    const setElectionId = (ElectionId: string | null): void => {
        localStorage.setItem("selected-election-event-id", ElectionId || "")
        setElection(ElectionId)
    }

    // Setup the context provider
    return (
        <ElectionContext.Provider
            value={{
                ElectionId: Election,
                setElectionId,
            }}
        >
            {props.children}
        </ElectionContext.Provider>
    )
}

export const useElectionStore: () => [string | null, (ElectionId: string | null) => void] = () => {
    const {ElectionId, setElectionId} = useContext(ElectionContext)

    return [ElectionId, setElectionId]
}
