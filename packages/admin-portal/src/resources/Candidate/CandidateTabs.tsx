import React from "react"
import {TabbedShowLayout, useRecordContext} from "react-admin"
import {Sequent_Backend_Candidate} from "../../gql/graphql"
import ElectionHeader from "../../components/ElectionHeader"
import {EditCandidateData} from "./EditCandidateData"
import {useTranslation} from "react-i18next"
import { translateElection } from '@sequentech/ui-essentials'

export const CandidateTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Candidate>()
    const {t, i18n} = useTranslation()

    return (
        <>
            <ElectionHeader
                title={translateElection(record, "name", i18n.language) ?? ""}
                subtitle="electionEventScreen.common.subtitle"
            />
            <TabbedShowLayout>
                <TabbedShowLayout.Tab label={t("electionScreen.tabs.data")}>
                    <EditCandidateData />
                </TabbedShowLayout.Tab>
            </TabbedShowLayout>
        </>
    )
}
