// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext, useState} from "react"

interface ElectionEventTallyContextProps {
    tallyId: string | null
    setTallyId: (tallyId: string | null) => void
}

const defaultElectionEventTallyContext: ElectionEventTallyContextProps = {
    tallyId: null,
    setTallyId: () => undefined,
}

export const ElectionEventTallyContext = createContext<ElectionEventTallyContextProps>(
    defaultElectionEventTallyContext
)

interface ElectionEventTallyContextProviderProps {
    children: JSX.Element
}

export const ElectionEventTallyContextProvider = (
    props: ElectionEventTallyContextProviderProps
) => {
    const [tally, setTally] = useState<string | null>(
        localStorage.getItem("selected-election-event-tally-id") || null
    )

    const setTallyId = (tallyId: string | null): void => {
        localStorage.setItem("selected-election-event-tally-id", tallyId?.toString() || "")
        setTally(tallyId)
    }

    return (
        <ElectionEventTallyContext.Provider
            value={{
                tallyId: tally,
                setTallyId,
            }}
        >
            {props.children}
        </ElectionEventTallyContext.Provider>
    )
}


export const useElectionEventTallyStore: () => [string | null, (tallyId: string | null) => void] = () => {
    const {tallyId, setTallyId} = useContext(ElectionEventTallyContext)
    return [tallyId, setTallyId]
}