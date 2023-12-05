// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {createContext, useState} from "react"

interface Context {
    lastCreatedResourceId: string | null
    setLastCreatedResourceId: (val: string | null) => void
}

const defaultContext: Context = {
    lastCreatedResourceId: null,
    setLastCreatedResourceId: () => undefined,
}

export const NewResourceContext = createContext<Context>(defaultContext)

export default function NewResourceContextProvider({children}: {children: React.ReactNode}) {
    const [lastCreatedResourceId, setLastCreatedResourceId] = useState<string | null>(null)

    return (
        <NewResourceContext.Provider
            value={{
                lastCreatedResourceId,
                setLastCreatedResourceId,
            }}
        >
            {children}
        </NewResourceContext.Provider>
    )
}
