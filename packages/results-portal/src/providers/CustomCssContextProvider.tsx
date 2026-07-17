// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {createContext, useContext, useMemo, useState} from "react"

interface CustomCssContextValue {
    customCss: string
    setCustomCss: (customCss: string) => void
}

const CustomCssContext = createContext<CustomCssContextValue>({
    customCss: "",
    setCustomCss: () => undefined,
})

export const CustomCssContextProvider: React.FC<{children: React.ReactNode}> = ({children}) => {
    const [customCss, setCustomCss] = useState("")
    const value = useMemo(() => ({customCss, setCustomCss}), [customCss])

    return <CustomCssContext.Provider value={value}>{children}</CustomCssContext.Provider>
}

export const useCustomCss = () => useContext(CustomCssContext)
