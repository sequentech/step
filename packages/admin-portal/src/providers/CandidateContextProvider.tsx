// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext, useState} from "react"

interface CandidateContextProps {
    CandidateId: string | null
    setCandidateId: (CandidateId: string | null) => void
}

const defaultCandidateContext: CandidateContextProps = {
    CandidateId: null,
    setCandidateId: () => undefined,
}

export const CandidateContext = createContext<CandidateContextProps>(defaultCandidateContext)

interface CandidateContextProviderProps {
    /**
     * The elements wrapped by the Candidate context.
     */
    children: React.ReactNode
}

export const CandidateContextProvider = (props: CandidateContextProviderProps) => {
    const [Candidate, setCandidate] = useState<string | null>(
        localStorage.getItem("selected-Candidate-event-id") || null
    )

    const setCandidateId = (CandidateId: string | null): void => {
        localStorage.setItem("selected-Candidate-event-id", CandidateId || "")
        setCandidate(CandidateId)
    }

    // Setup the context provider
    return (
        <CandidateContext.Provider
            value={{
                CandidateId: Candidate,
                setCandidateId,
            }}
        >
            {props.children}
        </CandidateContext.Provider>
    )
}

export const useCandidateStore: () => [
    string | null,
    (CandidateId: string | null) => void,
] = () => {
    const {CandidateId, setCandidateId} = useContext(CandidateContext)

    return [CandidateId, setCandidateId]
}
