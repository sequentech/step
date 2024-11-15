// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ETallyType} from "@/types/ceremonies"
import {IMiruTransmissionPackageData} from "@/types/miru"
import React, {createContext, useContext, useState} from "react"
import {Identifier} from "react-admin"

interface ElectionEventTallyContextProps {
    tallyId: string | null
    setTallyId: (tallyId: string | null, isTrustee?: boolean | undefined) => void
    isTrustee: boolean | undefined
    setCreatingFlag: (isCreating: ETallyType | null) => void
    isCreatingType: ETallyType | null
    setCreatedFlag: (isCreated: boolean) => void
    isCreated: boolean | undefined
    electionEventId: string | null
    setElectionEventId: (electionEventId: string | null) => void
    electionId: string | null
    setElectionId: (electionEventId: string | null) => void
    contestId: string | null
    setContestId: (electionEventId: string | null) => void
    miruAreaId: string | null
    setMiruAreaId: (electionEventId: string) => void
    selectedTallySessionData: IMiruTransmissionPackageData | null
    setSelectedTallySessionData: (tallySessionData: IMiruTransmissionPackageData | null) => void
    taskId: string | Identifier | null
    setTaskId: (tallyId: string | Identifier | null) => void
    customFilter: object
    setCustomFilter: (filter: object) => void
}

const defaultElectionEventTallyContext: ElectionEventTallyContextProps = {
    tallyId: null,
    setTallyId: () => undefined,
    isTrustee: false,
    setCreatingFlag: () => undefined,
    isCreatingType: null,
    setCreatedFlag: () => undefined,
    isCreated: undefined,
    electionEventId: null,
    setElectionEventId: () => undefined,
    electionId: null,
    setElectionId: () => undefined,
    contestId: null,
    setContestId: () => undefined,
    miruAreaId: null,
    setMiruAreaId: () => undefined,
    selectedTallySessionData: null,
    setSelectedTallySessionData: () => undefined,
    taskId: null,
    setTaskId: () => undefined,
    customFilter: {},
    setCustomFilter: () => undefined,
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
    const [isCreatingType, setIsCreatingType] = useState<ETallyType | null>(null)
    const [isCreated, setIsCreated] = useState<boolean | undefined>(undefined)
    const [task, setTask] = useState<string | Identifier | null>(null)
    const [miruAreaId, setMiruAreaId] = useState<string | null>(null)
    const [selectedTallySessionData, setSelectedTallySessionData] =
        useState<IMiruTransmissionPackageData | null>(null)
    const [customFilter, SetCustomFilter] = useState<any>({})

    const [electionEventId, setElectionEventId] = useState<string | null>(null)
    const [electionId, setElectionId] = useState<string | null>(null)
    const [contestId, setContestId] = useState<string | null>(null)

    const setTallyId = (tallyId: string | null, isTrustee?: boolean | undefined): void => {
        setTally(tallyId)
        setIsTrustee(isTrustee || false)
    }

    const setCreatingFlag = (isCreating: ETallyType | null): void => {
        setIsCreatingType(isCreating)
    }

    const setCreatedFlag = (isCreated: boolean): void => {
        setIsCreated(isCreated)
    }

    const setTaskId = (value: string | Identifier | null): void => {
        setTask(value)
    }

    const setCustomFilter = (filter: object): void => {
        SetCustomFilter(filter)
    }

    return (
        <ElectionEventTallyContext.Provider
            value={{
                tallyId: tally,
                setTallyId,
                isTrustee,
                isCreatingType,
                setCreatingFlag,
                isCreated,
                setCreatedFlag,
                electionEventId,
                setElectionEventId,
                electionId,
                setElectionId,
                contestId,
                setContestId,
                miruAreaId,
                setMiruAreaId,
                selectedTallySessionData,
                setSelectedTallySessionData,
                setTaskId,
                taskId: task,
                customFilter,
                setCustomFilter,
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
    isCreatingType: ETallyType | null
    setCreatingFlag: (isCreating: ETallyType | null) => void
    setCreatedFlag: (isCreated: boolean) => void
    isCreated: boolean | undefined
    electionEventId: string | null
    setElectionEventId: (electionEventId: string | null) => void
    electionId: string | null
    setElectionId: (electionEventId: string | null) => void
    contestId: string | null
    setContestId: (electionEventId: string | null) => void
    setSelectedTallySessionData: (tallySessionData: IMiruTransmissionPackageData | null) => void
    selectedTallySessionData: IMiruTransmissionPackageData | null
    miruAreaId: string | null
    setMiruAreaId: (electionEventId: string) => void
    taskId: string | Identifier | null
    setTaskId: (tallyId: string | Identifier | null) => void
    customFilter: object
    setCustomFilter: (filter: object) => void
} = () => {
    const {
        tallyId,
        setTallyId,
        isTrustee,
        isCreatingType,
        setCreatingFlag,
        isCreated,
        setCreatedFlag,
        electionEventId,
        setElectionEventId,
        electionId,
        setElectionId,
        contestId,
        setContestId,
        miruAreaId,
        setMiruAreaId,
        selectedTallySessionData,
        setSelectedTallySessionData,
        taskId,
        setTaskId,
        customFilter,
        setCustomFilter,
    } = useContext(ElectionEventTallyContext)
    return {
        tallyId,
        setTallyId,
        isTrustee,
        isCreatingType,
        setCreatingFlag,
        isCreated,
        setCreatedFlag,
        electionEventId,
        setElectionEventId,
        electionId,
        setElectionId,
        contestId,
        setContestId,
        miruAreaId,
        setMiruAreaId,
        selectedTallySessionData,
        setSelectedTallySessionData,
        taskId,
        setTaskId,
        customFilter,
        setCustomFilter,
    }
}
