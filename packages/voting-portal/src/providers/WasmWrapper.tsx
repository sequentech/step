// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useWasm, WasmContextProvider, WasmStatus} from "@sequentech/ui-core"
import {Loader} from "@sequentech/ui-essentials"

export const WasmGate: React.FC<React.PropsWithChildren> = ({children}) => {
    const {status} = useWasm()

    return WasmStatus.READY === status ? <>{children}</> : <Loader />
}

export const WasmWrapper: React.FC<React.PropsWithChildren> = ({children}) => (
    <WasmContextProvider>
        <WasmGate>{children}</WasmGate>
    </WasmContextProvider>
)
