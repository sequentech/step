// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {createContext, useState} from "react"

interface Context {
    hasJustCreateNewResource: boolean
    setHasJustCreateNewResource: (val: boolean) => void
}

const defaultContext: Context = {
    hasJustCreateNewResource: false,
    setHasJustCreateNewResource: () => undefined,
}

export const NewResourceContext = createContext<Context>(defaultContext)

export default function NewResourceContextProvider({children}: {children: React.ReactNode}) {
    const [hasJustCreateNewResource, setHasJustCreateNewResource] = useState<boolean>(false)

    return (
        <NewResourceContext.Provider
            value={{
                hasJustCreateNewResource,
                setHasJustCreateNewResource,
            }}
        >
            {children}
        </NewResourceContext.Provider>
    )
}
