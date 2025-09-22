// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ETallyType} from "@/types/ceremonies"
import {IMiruTransmissionPackageData} from "@/types/miru"
import {LSSelections} from "@/types/storage"
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
    setElectionEventIdFlag: (electionEventId: string | null) => void
    electionId: string | null
    setElectionIdFlag: (electionId: string | null) => void
    contestId: string | null
    setContestIdFlag: (contestId: string | null) => void
    getContestIdFlag: () => string | null
    candidateId: string | null
    setCandidateIdFlag: (candidateId: string | null) => void
    getCandidateIdFlag: () => string | null
    miruAreaId: string | null
    setMiruAreaId: (miruAreaId: string) => void
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
    setElectionEventIdFlag: () => undefined,
    electionId: null,
    setElectionIdFlag: () => undefined,
    contestId: null,
    setContestIdFlag: () => undefined,
    getContestIdFlag: () => null,
    candidateId: null,
    setCandidateIdFlag: () => undefined,
    getCandidateIdFlag: () => null,
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
    const [candidateId, setCandidateId] = useState<string | null>(null)

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

    const setElectionEventIdFlag = (electionEventId: string | null): void => {
        if (electionEventId) {
            setElectionEventId(electionEventId)
            localStorage.setItem(LSSelections.ELECTION_EVENT, electionEventId)
        } else {
            setElectionEventId(null)
            localStorage.removeItem(LSSelections.ELECTION_EVENT)
        }
    }

    const setElectionIdFlag = (electionId: string | null): void => {
        if (electionId) {
            setElectionId(electionId)
            localStorage.setItem(LSSelections.ELECTION, electionId)
        } else {
            setElectionId(null)
            localStorage.removeItem(LSSelections.ELECTION)
        }
    }

    const setContestIdFlag = (contestId: string | null): void => {
        if (contestId) {
            setContestId(contestId)
            localStorage.setItem(LSSelections.CONTEST, contestId)
        } else {
            setContestId(null)
            localStorage.removeItem(LSSelections.CONTEST)
        }
    }

    const setCandidateIdFlag = (candidateId: string | null): void => {
        if (candidateId) {
            setCandidateId(contestId)
            localStorage.setItem(LSSelections.CANDIDATE, candidateId)
        } else {
            setCandidateId(null)
            localStorage.removeItem(LSSelections.CANDIDATE)
        }
    }

    const getContestIdFlag = (): string | null => {
        return localStorage.getItem(LSSelections.CONTEST) ?? null
    }
    const getCandidateIdFlag = (): string | null => {
        return localStorage.getItem(LSSelections.CANDIDATE) ?? null
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
                setElectionEventIdFlag,
                electionId,
                setElectionIdFlag,
                contestId,
                setContestIdFlag,
                getContestIdFlag,
                candidateId,
                setCandidateIdFlag,
                getCandidateIdFlag,
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
    creatingType: ETallyType | null
    setCreatingFlag: (isCreating: ETallyType | null) => void
    setCreatedFlag: (isCreated: boolean) => void
    isCreated: boolean | undefined
    electionEventId: string | null
    setElectionEventIdFlag: (electionEventId: string | null) => void
    electionId: string | null
    setElectionIdFlag: (electionId: string | null) => void
    contestId: string | null
    setContestIdFlag: (contestId: string | null) => void
    getContestIdFlag: () => string | null
    candidateId: string | null
    setCandidateIdFlag: (candidateId: string | null) => void
    getCandidateIdFlag: () => string | null
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
        setElectionEventIdFlag,
        electionId,
        setElectionIdFlag,
        contestId,
        setContestIdFlag,
        getContestIdFlag,
        candidateId,
        setCandidateIdFlag,
        getCandidateIdFlag,
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
        creatingType: isCreatingType,
        setCreatingFlag,
        isCreated,
        setCreatedFlag,
        electionEventId,
        setElectionEventIdFlag,
        electionId,
        setElectionIdFlag,
        contestId,
        setContestIdFlag,
        getContestIdFlag,
        candidateId,
        setCandidateIdFlag,
        getCandidateIdFlag,
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
