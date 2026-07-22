import React, {createContext, useContext, useEffect, useState} from "react"
import {IvrEmulatorApi, IvrEmulatorError, loadIvrEmulator} from "@/services/IvrEmulator"

export enum IvrStatus {
    UNAVAILABLE = "unavailable",
    LOADING = "loading",
    READY = "ready",
    ERROR = "error",
}

export interface IvrEmulatorContextType {
    status: IvrStatus
    api?: IvrEmulatorApi
}
export const IvrEmulatorContext = createContext<IvrEmulatorContextType | undefined>(undefined)
export const IvrEmulatorContextProvider: React.FC<{children: React.ReactNode}> = ({children}) => {
    const [status, setStatus] = useState<IvrStatus>(IvrStatus.LOADING)
    const [api, setApi] = useState<IvrEmulatorApi | undefined>(undefined)
    const url = "/wasm/ivr_emulator_wasm_bg.wasm"

    useEffect(() => {
        loadIvrEmulator(url)
            ?.then((resolvedApi) => {
                setStatus(IvrStatus.READY)
                setApi(resolvedApi)
            })
            .catch((e) => {
                if (e instanceof IvrEmulatorError && e.operation === "fetch") {
                    setStatus(IvrStatus.UNAVAILABLE)
                } else {
                    setStatus(IvrStatus.ERROR)
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
