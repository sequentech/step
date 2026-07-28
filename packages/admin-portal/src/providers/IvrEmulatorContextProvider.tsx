// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext, useEffect, useState} from "react"
import {IvrEmulatorApi, IvrEmulatorError, loadIvrEmulator} from "@/services/IvrEmulator"
import {SettingsContext} from "@/providers/SettingsContextProvider"

export enum IvrApiStatus {
    UNAVAILABLE = "unavailable",
    LOADING = "loading",
    READY = "ready",
    ERROR = "error",
}

export interface IvrEmulatorContextType {
    status: IvrApiStatus
    api?: IvrEmulatorApi
}

export const IvrEmulatorContext = createContext<IvrEmulatorContextType | undefined>(undefined)
export const IvrEmulatorContextProvider: React.FC<{children: React.ReactNode}> = ({children}) => {
    const [status, setStatus] = useState<IvrApiStatus>(IvrApiStatus.LOADING)
    const [api, setApi] = useState<IvrEmulatorApi | undefined>(undefined)
    const {globalSettings} = useContext(SettingsContext)
    const url = globalSettings.IVR_EMULATOR_BASE_URL
        ? globalSettings.IVR_EMULATOR_BASE_URL
        : "/wasm/ivr_emulator_wasm"

    useEffect(() => {
        loadIvrEmulator(url)
            ?.then((resolvedApi) => {
                setStatus(IvrApiStatus.READY)
                setApi(resolvedApi)
            })
            .catch((e) => {
                if (e instanceof IvrEmulatorError && e.operation === "fetch") {
                    setStatus(IvrApiStatus.UNAVAILABLE)
                } else {
                    setStatus(IvrApiStatus.ERROR)
                }
            })
    }, [])
    const value = {status, api}

    return <IvrEmulatorContext.Provider value={value}>{children}</IvrEmulatorContext.Provider>
}

export const useIvrEmulator = (): IvrEmulatorContextType => {
    const emulator = useContext(IvrEmulatorContext)
    if (!emulator) {
        throw new Error("useIvrEmulator can't be used outside IvrEmulatorContext")
    }
    return emulator
}
