// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import {ShowBase} from "react-admin"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {ElectionEventTabs} from "./ElectionEventTabs"

export const ElectionEventBaseTabs: React.FC = () => {
    const {globalSettings} = useContext(SettingsContext)
    return (
        <ShowBase queryOptions={{refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS}}>
            <ElectionEventTabs />
        </ShowBase>
    )
}
