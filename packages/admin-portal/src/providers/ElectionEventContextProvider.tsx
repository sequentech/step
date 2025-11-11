// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext, useState} from "react"

interface ElectionEventContextProps {
    ElectionEventId: string | null
    setElectionEventId: (ElectionEventId: string | null) => void
}

const defaultElectionEventContext: ElectionEventContextProps = {
    ElectionEventId: null,
    setElectionEventId: () => undefined,
}

export const ElectionEventContext = createContext<ElectionEventContextProps>(
    defaultElectionEventContext
)

interface ElectionEventContextProviderProps {
    /**
     * The elements wrapped by the ElectionEvent context.
     */
    children: React.ReactNode
}

export const ElectionEventContextProvider = (props: ElectionEventContextProviderProps) => {
    const [ElectionEvent, setElectionEvent] = useState<string | null>(
        localStorage.getItem("selected-election-event-id") || null
    )

    const setElectionEventId = (ElectionEventId: string | null): void => {
        localStorage.setItem("selected-election-event-id", ElectionEventId || "")
        setElectionEvent(ElectionEventId)
    }

    // Setup the context provider
    return (
        <ElectionEventContext.Provider
            value={{
                ElectionEventId: ElectionEvent,
                setElectionEventId,
            }}
        >
            {props.children}
        </ElectionEventContext.Provider>
    )
}

export const useElectionEventStore: () => [
    string | null,
    (ElectionEventId: string | null) => void,
] = () => {
    const {ElectionEventId, setElectionEventId} = useContext(ElectionEventContext)

    return [ElectionEventId, setElectionEventId]
}
