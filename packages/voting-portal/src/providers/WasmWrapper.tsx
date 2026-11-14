// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useWasm, WasmContextProvider, WasmStatus} from "@sequentech/ui-core"
import NeutralLoader from "../components/NeutralLoader/NeutralLoader"

export const WasmGate: React.FC<React.PropsWithChildren> = ({children}) => {
    const {status} = useWasm()

    return WasmStatus.READY === status ? <>{children}</> : <NeutralLoader />
}

export const WasmWrapper: React.FC<React.PropsWithChildren> = ({children}) => (
    <WasmContextProvider>
        <WasmGate>{children}</WasmGate>
    </WasmContextProvider>
)
