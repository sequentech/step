// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext, useState} from "react"

interface PublishContextProps {
    electionEventId: string
    electionId: string | null
    ballotPublicationId: string | null
    setEvent: (newElectionEventId: string, newElectionId: string | null) => void
    setBallotPublicationId: (id: string | null) => void
}

const defaultPublishContext: PublishContextProps = {
    electionEventId: "",
    electionId: null,
    ballotPublicationId: null,
    setEvent: () => undefined,
    setBallotPublicationId: () => undefined,
}

export const PublishContext = createContext<PublishContextProps>(defaultPublishContext)

interface PublishContextProviderProps {
    /**
     * The elements wrapped by the publish context.
     */
    children: React.ReactNode
}

export const PublishContextProvider = (props: PublishContextProviderProps) => {
    const [electionEventId, setElectionEventId] = useState<string>(
        defaultPublishContext.electionEventId
    )
    const [electionId, setElectionId] = useState<string | null>(defaultPublishContext.electionId)
    const [ballotPublicationId, setBallotPublicationId] = useState<string | null>(
        defaultPublishContext.ballotPublicationId
    )

    const setEvent = (newElectionEventId: string, newElectionId: string | null): void => {
        if (newElectionEventId !== electionEventId || newElectionId !== electionId) {
            setBallotPublicationId(null)
        }
        setElectionEventId(newElectionEventId)
        setElectionId(newElectionId)
    }

    // Setup the context provider
    return (
        <PublishContext.Provider
            value={{
                electionEventId,
                electionId,
                ballotPublicationId,
                setEvent,
                setBallotPublicationId,
            }}
        >
            {props.children}
        </PublishContext.Provider>
    )
}
