// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {createContext, useState} from "react"

export type ResourceType =
    | "sequent_backend_election_event"
    | "sequent_backend_election"
    | "sequent_backend_contest"
    | "sequent_backend_candidate"

export type Resource = {
    id: string
    type: ResourceType
}

interface Context {
    lastCreatedResource: Resource | null
    setLastCreatedResource: (val: Resource | null) => void
}

const defaultContext: Context = {
    lastCreatedResource: null,
    setLastCreatedResource: () => undefined,
}

export const NewResourceContext = createContext<Context>(defaultContext)

export default function NewResourceContextProvider({children}: {children: React.ReactNode}) {
    const [lastCreatedResource, setLastCreatedResource] = useState<Resource | null>(null)

    return (
        <NewResourceContext.Provider
            value={{
                lastCreatedResource: lastCreatedResource,
                setLastCreatedResource: setLastCreatedResource,
            }}
        >
            {children}
        </NewResourceContext.Provider>
    )
}
