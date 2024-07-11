// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {ShowBase} from "react-admin"
import {ElectionEventTabs} from "./ElectionEventTabs"
import {ViewModeContextProvider} from "@/providers/ViewModeContextProvider"

export const ElectionEventBaseTabs: React.FC = () => {
    return (
        <ShowBase>
            <ViewModeContextProvider>
                <ElectionEventTabs />
            </ViewModeContextProvider>
        </ShowBase>
    )
}
