// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {TabbedShowLayout, useRecordContext} from "react-admin"
import {Sequent_Backend_Candidate} from "../../gql/graphql"
import ElectionHeader from "../../components/ElectionHeader"
import {EditCandidateData} from "./EditCandidateData"

export const CandidateTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Candidate>()

    return (
        <>
            <ElectionHeader title={record?.name || ""} subtitle="candidateScreen.common.subtitle" />
            <TabbedShowLayout>
                <TabbedShowLayout.Tab label="Data">
                    <EditCandidateData record={record} />
                </TabbedShowLayout.Tab>
            </TabbedShowLayout>
        </>
    )
}
