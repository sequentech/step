import {ETallyType} from "@/types/ceremonies"
import {IMiruTransmissionPackageData} from "@/types/miru"
import React, {createContext, useContext, useState} from "react"
import {Identifier} from "react-admin"

interface ElectionEventTallyContextProps {
    tallyId: string | null
    setTallyId: (tallyId: string | null, isTrustee?: boolean | undefined) => void
    isTrustee: boolean | undefined
    setCreatingFlag: (isCreating: ETallyType | null) => void
    isCreating: ETallyType | null
    setCreatedFlag: (isCreated: boolean) => void
    isCreated: boolean | undefined
    electionEventId: string | null
    setElectionEventId: (electionEventId: string) => void
    miruAreaId: string | null
    setMiruAreaId: (electionEventId: string) => void
    selectedTallySessionData: IMiruTransmissionPackageData | null
    setSelectedTallySessionData: (tallySessionData: IMiruTransmissionPackageData | null) => void
    taskId: string | Identifier | null
    setTaskId: (taskId: string | Identifier | null) => void
}

const defaultElectionEventTallyContext: ElectionEventTallyContextProps = {
    tallyId: null,
    setTallyId: () => undefined,
    isTrustee: false,
    setCreatingFlag: () => undefined,
    isCreating: null,
    setCreatedFlag: () => undefined,
    isCreated: undefined,
    electionEventId: null,
    setElectionEventId: () => undefined,
    miruAreaId: null,
    setMiruAreaId: () => undefined,
    selectedTallySessionData: null,
    setSelectedTallySessionData: () => undefined,
    taskId: null,
    setTaskId: () => undefined,
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
    const [isCreating, setIsCreating] = useState<ETallyType | null>(null)
    const [isCreated, setIsCreated] = useState<boolean | undefined>(undefined)
    const [electionEventId, setElectionEventId] = useState<string | null>(null)
    const [task, setTask] = useState<string | Identifier | null>(null)
    const [miruAreaId, setMiruAreaId] = useState<string | null>(null)
    const [selectedTallySessionData, setSelectedTallySessionData] =
        useState<IMiruTransmissionPackageData | null>(null)

    const setTallyId = (tallyId: string | null, isTrustee?: boolean | undefined): void => {
        setTally(tallyId)
        setIsTrustee(isTrustee || false)
    }

    const setCreatingFlag = (isCreating: ETallyType | null): void => {
        setIsCreating(isCreating)
    }

    const setCreatedFlag = (isCreated: boolean): void => {
        setIsCreated(isCreated)
    }

    const setTaskId = (value: string | Identifier | null): void => {
        setTask(value)
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
                setTaskId,
                taskId: task,
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
    isCreating: ETallyType | null
    setCreatingFlag: (isCreating: ETallyType | null) => void
    setCreatedFlag: (isCreated: boolean) => void
    isCreated: boolean | undefined
    electionEventId: string | null
    setElectionEventId: (electionEventId: string) => void
    setSelectedTallySessionData: (tallySessionData: IMiruTransmissionPackageData | null) => void
    selectedTallySessionData: IMiruTransmissionPackageData | null
    miruAreaId: string | null
    setMiruAreaId: (electionEventId: string) => void
    taskId: string | Identifier | null
    setTaskId: (taskId: string | Identifier | null) => void
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
        taskId,
        setTaskId,
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
        taskId,
        setTaskId,
    }
}
