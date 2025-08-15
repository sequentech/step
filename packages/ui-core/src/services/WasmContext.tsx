// SPDX-FileCopyrightText: 2024-2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext, useEffect, useState} from "react"
import {initCore} from "./wasm"

// Define the possible states of our WASM module
export enum WasmStatus {
    LOADING = "loading",
    READY = "ready",
    ERROR = "error",
}

// The shape of our context data
export interface WasmContextType {
    status: WasmStatus
}

// Create the context with a default value
export const WasmContext = createContext<WasmContextType | undefined>(undefined)

// Create the Provider component
export const WasmContextProvider: React.FC<{children: React.ReactNode}> = ({children}) => {
    const [status, setStatus] = useState<WasmStatus>(WasmStatus.LOADING)

    useEffect(() => {
        // We only want this effect to run once when the component mounts
        initCore()
            .then(() => {
                setStatus(WasmStatus.READY)
            })
            .catch((error) => {
                console.error("Error loading sequent-core: " + error)
                setStatus(WasmStatus.ERROR)
            })
    }, []) // Empty dependency array ensures it runs only once

    const value = {status}

    return <WasmContext.Provider value={value}>{children}</WasmContext.Provider>
}

// Create a custom hook for easy consumption
export const useWasm = () => {
    const context = useContext(WasmContext)
    if (context === undefined) {
        throw new Error("useWasm must be used within a WasmProvider")
    }
    return context
}
