// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {Identifier, TabbedShowLayout, useRecordContext} from "react-admin"
import {Sequent_Backend_Contest} from "../../gql/graphql"
import ElectionHeader from "../../components/ElectionHeader"
import {EditContestData} from "./EditContestData"

export const ContestTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Contest>()

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
