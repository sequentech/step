// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
// Adjust the import path to where your hook file is located
import {DatabaseContext, useDatabaseManager} from "../hooks/useSQLiteDatabase"

export const DatabaseProvider: React.FC<{children: React.ReactNode}> = ({children}) => {
    // useDatabaseManager creates the state (the map of databases) and the functions to modify it.
    const manager = useDatabaseManager()

    // The Provider component makes this state and these functions available to any child component.
    return (
        <DatabaseContext.Provider value={manager.contextValue}>{children}</DatabaseContext.Provider>
    )
}
