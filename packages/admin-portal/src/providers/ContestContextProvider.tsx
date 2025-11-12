// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext, useState} from "react"

interface ContestContextProps {
    ContestId: string | null
    setContestId: (ContestId: string | null) => void
}

const defaultContestContext: ContestContextProps = {
    ContestId: null,
    setContestId: () => undefined,
}

export const ContestContext = createContext<ContestContextProps>(defaultContestContext)

interface ContestContextProviderProps {
    /**
     * The elements wrapped by the Contest context.
     */
    children: React.ReactNode
}

export const ContestContextProvider = (props: ContestContextProviderProps) => {
    const [Contest, setContest] = useState<string | null>(
        localStorage.getItem("selected-Contest-event-id") || null
    )

    const setContestId = (ContestId: string | null): void => {
        localStorage.setItem("selected-Contest-event-id", ContestId || "")
        setContest(ContestId)
    }

    // Setup the context provider
    return (
        <ContestContext.Provider
            value={{
                ContestId: Contest,
                setContestId,
            }}
        >
            {props.children}
        </ContestContext.Provider>
    )
}

export const useContestStore: () => [string | null, (ContestId: string | null) => void] = () => {
    const {ContestId, setContestId} = useContext(ContestContext)

    return [ContestId, setContestId]
}
