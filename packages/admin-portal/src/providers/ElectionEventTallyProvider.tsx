// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext, useState} from "react"

interface ElectionEventTallyContextProps {
    tallyId: string | null
    setTallyId: (tallyId: string | null, isTrustee?: boolean | undefined) => void
    setCreatingFlag: (isCreating: boolean) => void
    isTrustee: boolean | undefined
    isCreating: boolean | undefined
}

const defaultElectionEventTallyContext: ElectionEventTallyContextProps = {
    tallyId: null,
    setTallyId: () => undefined,
    setCreatingFlag: () => undefined,
    isTrustee: false,
    isCreating: false,
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
    const [isCreating, setIsCreating] = useState<boolean>(false)

    const setTallyId = (tallyId: string | null, isTrustee?: boolean | undefined): void => {
        localStorage.setItem("selected-election-event-tally-id", tallyId?.toString() || "")
        setTally(tallyId)
        setIsTrustee(isTrustee || false)
    }

    const setCreatingFlag = (isCreating: boolean): void => {
        setIsCreating(isCreating)
    }

    return (
        <ElectionEventTallyContext.Provider
            value={{
                tallyId: tally,
                setTallyId,
                isTrustee,
                isCreating,
                setCreatingFlag,
            }}
        >
            {props.children}
        </ElectionEventTallyContext.Provider>
    )
}

export const useElectionEventTallyStore: () => {
    tallyId: string | null
    setTallyId: (tallyId: string | null, isTrustee?: boolean | undefined) => void
    setCreatingFlag: (isCreating: boolean) => void
    isTrustee: boolean | undefined
    isCreating: boolean | undefined
} = () => {
    const {tallyId, setTallyId, isTrustee, isCreating, setCreatingFlag} =
        useContext(ElectionEventTallyContext)
    return {tallyId, setTallyId, isTrustee, isCreating, setCreatingFlag}
}
