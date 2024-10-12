// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {IMiruTransmissionPackageData} from "@/types/miru"
import React, {createContext, useContext, useState} from "react"

interface ElectionEventTallyContextProps {
    tallyId: string | null
    setTallyId: (tallyId: string | null, isTrustee?: boolean | undefined) => void
    isTrustee: boolean | undefined
    setCreatingFlag: (isCreating: boolean) => void
    isCreating: boolean | undefined
    setCreatedFlag: (isCreating: boolean) => void
    isCreated: boolean | undefined
    electionEventId: string | null
    setElectionEventId: (electionEventId: string) => void
    miruAreaId: string | null
    setMiruAreaId: (electionEventId: string) => void
    selectedTallySessionData: IMiruTransmissionPackageData | null
    setSelectedTallySessionData: (tallySessionDate: IMiruTransmissionPackageData | null) => void
}

const defaultElectionEventTallyContext: ElectionEventTallyContextProps = {
    tallyId: null,
    setTallyId: () => undefined,
    isTrustee: false,
    setCreatingFlag: () => undefined,
    isCreating: false,
    setCreatedFlag: () => undefined,
    isCreated: false,
    electionEventId: null,
    setElectionEventId: () => undefined,
    miruAreaId: null,
    setMiruAreaId: () => undefined,
    selectedTallySessionData: null,
    setSelectedTallySessionData: () => undefined,
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
    const [tally, setTally] = useState<string | null>(null)
    const [isTrustee, setIsTrustee] = useState<boolean>(false)
    const [isCreating, setIsCreating] = useState<boolean>(false)
    const [isCreated, setIsCreated] = useState<boolean>(false)
    const [electionEventId, setElectionEventId] = useState<string | null>(null)
    const [miruAreaId, setMiruAreaId] = useState<string | null>(null)
    const [selectedTallySessionData, setSelectedTallySessionData] =
        useState<IMiruTransmissionPackageData | null>(null)

    const setTallyId = (tallyId: string | null, isTrustee?: boolean | undefined): void => {
        sessionStorage.setItem("selected-election-event-tally-id", tallyId?.toString() || "")
        setTally(tallyId)
        setIsTrustee(isTrustee || false)
    }

    const setCreatingFlag = (isCreating: boolean): void => {
        setIsCreating(isCreating)
    }

    const setCreatedFlag = (isCreated: boolean): void => {
        setIsCreated(isCreating)
    }

    return (
        <ElectionEventTallyContext.Provider
            value={{
                tallyId: tally,
                setTallyId,
                isTrustee,
                isCreating,
                setCreatingFlag,
                isCreated,
                setCreatedFlag,
                electionEventId,
                setElectionEventId,
                miruAreaId,
                setMiruAreaId,
                selectedTallySessionData,
                setSelectedTallySessionData,
            }}
        >
            {props.children}
        </ElectionEventTallyContext.Provider>
    )
}

export const useElectionEventTallyStore: () => {
    tallyId: string | null
    setTallyId: (tallyId: string | null, isTrustee?: boolean | undefined) => void
    isTrustee: boolean | undefined
    setCreatingFlag: (isCreating: boolean) => void
    isCreating: boolean | undefined
    setCreatedFlag: (isCreating: boolean) => void
    isCreated: boolean | undefined
    electionEventId: string | null
    setElectionEventId: (electionEventId: string) => void
    setSelectedTallySessionData: (tallySessionData: IMiruTransmissionPackageData | null) => void
    selectedTallySessionData: IMiruTransmissionPackageData | null
    miruAreaId: string | null
    setMiruAreaId: (electionEventId: string) => void
} = () => {
    const {
        tallyId,
        setTallyId,
        isTrustee,
        isCreating,
        setCreatingFlag,
        isCreated,
        setCreatedFlag,
        electionEventId,
        setElectionEventId,
        miruAreaId,
        setMiruAreaId,
        selectedTallySessionData,
        setSelectedTallySessionData,
    } = useContext(ElectionEventTallyContext)
    return {
        tallyId,
        setTallyId,
        isTrustee,
        isCreating,
        setCreatingFlag,
        isCreated,
        setCreatedFlag,
        electionEventId,
        setElectionEventId,
        miruAreaId,
        setMiruAreaId,
        selectedTallySessionData,
        setSelectedTallySessionData,
    }
}
