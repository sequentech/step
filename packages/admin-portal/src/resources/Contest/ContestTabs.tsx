// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Identifier, TabbedShowLayout, useRecordContext} from "react-admin"
import {Sequent_Backend_Contest} from "../../gql/graphql"
import ElectionHeader from "../../components/ElectionHeader"
import {EditContestData} from "./EditContestData"
import {CircularProgress} from "@mui/material"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"

export const ContestTabs: React.FC = () => {
    const aliasRenderer = useAliasRenderer()
    const record = useRecordContext<Sequent_Backend_Contest>()
    if (!record) {
        return <CircularProgress />
    }

    return (
        <>
            <ElectionHeader
                title={aliasRenderer(record)}
                subtitle="contestScreen.common.subtitle"
            />
            <TabbedShowLayout>
                <TabbedShowLayout.Tab label="Data">
                    <EditContestData />
                </TabbedShowLayout.Tab>
            </TabbedShowLayout>
        </>
    )
}
