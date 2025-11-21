// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {Identifier, TabbedShowLayout, useRecordContext} from "react-admin"
import {Sequent_Backend_Contest} from "../../gql/graphql"
import ElectionHeader from "../../components/ElectionHeader"
import {EditContestData} from "./EditContestData"
import {CircularProgress} from "@mui/material"

export const ContestTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Contest>()
    if (!record) {
        return <CircularProgress />
    }

    return (
        <>
            <ElectionHeader title={record?.name || ""} subtitle="contestScreen.common.subtitle" />
            <TabbedShowLayout>
                <TabbedShowLayout.Tab label="Data">
                    <EditContestData />
                </TabbedShowLayout.Tab>
            </TabbedShowLayout>
        </>
    )
}
