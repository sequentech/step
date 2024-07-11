// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {createContext, useState} from "react"

export enum ViewMode {
    Edit,
    View,
    List,
}

interface ViewModeContextProps {
    viewMode: ViewMode
    setViewMode: (viewMode: ViewMode) => void
}

const defaultViewModeContext: ViewModeContextProps = {
    viewMode: ViewMode.List,
    setViewMode: () => undefined,
}

export const ViewModeContext = createContext<ViewModeContextProps>(defaultViewModeContext)

interface ViewModeContextProviderProps {
    /**
     * The elements wrapped by the viewMode context.
     */
    children: JSX.Element
}

export const ViewModeContextProvider = (props: ViewModeContextProviderProps) => {
    const [viewMode, setViewMode] = useState(ViewMode.List)

    return (
        <ViewModeContext.Provider value={{viewMode, setViewMode}}>
            {props.children}
        </ViewModeContext.Provider>
    )
}
