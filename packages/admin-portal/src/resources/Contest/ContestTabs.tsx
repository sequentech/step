import React from "react"
import {TabbedShowLayout, useRecordContext} from "react-admin"
import {Sequent_Backend_Contest} from "../../gql/graphql"
import ElectionHeader from "../../components/ElectionHeader"
import {EditContestData} from "./EditContestData"

export const ContestTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Contest>()

    return (
        <>
            <ElectionHeader
                title={record?.name || ""}
                subtitle="electionEventScreen.common.subtitle"
            />
            <TabbedShowLayout>
                <TabbedShowLayout.Tab label="Data">
                    <EditContestData />
                </TabbedShowLayout.Tab>
            </TabbedShowLayout>
        </>
    )
}
