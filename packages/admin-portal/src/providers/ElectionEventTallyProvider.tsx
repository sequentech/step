// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext, useState} from "react"

interface ElectionEventTallyContextProps {
    tallyId: string | null
    setTallyId: (tallyId: string | null, isTrustee?: boolean | undefined) => void
    isTrustee: boolean | undefined
}

const defaultElectionEventTallyContext: ElectionEventTallyContextProps = {
    tallyId: null,
    setTallyId: () => undefined,
    isTrustee: false,
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
    const [isTrustee, setIsTrustee] = useState<boolean>(false)

    const setTallyId = (tallyId: string | null, isTrustee?: boolean | undefined): void => {
        localStorage.setItem("selected-election-event-tally-id", tallyId?.toString() || "")
        setTally(tallyId)
        setIsTrustee(isTrustee || false)
    }

    return (
        <ElectionEventTallyContext.Provider
            value={{
                tallyId: tally,
                setTallyId,
                isTrustee,
            }}
        >
            {props.children}
        </ElectionEventTallyContext.Provider>
    )
}


export const useElectionEventTallyStore: () => [string | null, (tallyId: string | null, isTrustee?: boolean | undefined) => void, boolean | undefined] = () => {
    const {tallyId, setTallyId, isTrustee} = useContext(ElectionEventTallyContext)
    return [tallyId, setTallyId, isTrustee]
}