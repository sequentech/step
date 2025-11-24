// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect} from "react"
import {TabbedShowLayout, useRecordContext} from "react-admin"
import {Sequent_Backend_Candidate} from "../../gql/graphql"
import ElectionHeader from "../../components/ElectionHeader"
import {EditCandidateData} from "./EditCandidateData"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"

export const CandidateTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Candidate>()
    const {setCandidateIdFlag, setContestIdFlag, setElectionEventIdFlag} =
        useElectionEventTallyStore()

    useEffect(() => {
        if (record) {
            setCandidateIdFlag(record.id)
            setContestIdFlag(record.contest_id)
            setElectionEventIdFlag(record.election_event_id)
        }
    }, [record])

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
