// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect} from "react"
import {TabbedShowLayout, useRecordContext} from "react-admin"
import {Sequent_Backend_Candidate} from "../../gql/graphql"
import ElectionHeader from "../../components/ElectionHeader"
import {EditCandidateData} from "./EditCandidateData"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {configuredLanguage, useInheritedLanguage} from "@/hooks/usePresentationName"

export const CandidateTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Candidate>()
    const aliasRenderer = useAliasRenderer()
    const inheritedLanguage = useInheritedLanguage(record)
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
            <ElectionHeader
                title={aliasRenderer(record, configuredLanguage(record) ?? inheritedLanguage) || ""}
                subtitle="candidateScreen.common.subtitle"
            />
            <TabbedShowLayout>
                <TabbedShowLayout.Tab label="Data">
                    <EditCandidateData record={record} />
                </TabbedShowLayout.Tab>
            </TabbedShowLayout>
        </>
    )
}
