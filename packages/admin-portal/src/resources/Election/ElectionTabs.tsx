import React from "react"

import { useTranslation } from "react-i18next"
import { TabbedShowLayout, useRecordContext } from "react-admin"

import ElectionHeader from "../../components/ElectionHeader"
import DashboardElection from "@/components/dashboard/election/Dashboard"

import { Publish } from "../Publish/Publish"
import { EditElectionData } from "./ElectionData"
import { Sequent_Backend_Election } from "../../gql/graphql"

export const ElectionTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election>()
    const {t} = useTranslation()

    return (
        <>
            <ElectionHeader title={record?.name} subtitle="electionEventScreen.common.subtitle" />
            <TabbedShowLayout>
                <TabbedShowLayout.Tab label={t("electionScreen.tabs.dashboard")}>
                    <DashboardElection />
                </TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label={t("electionScreen.tabs.data")}>
                    <EditElectionData />
                </TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label="Dashboard">a</TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label={t("electionScreen.tabs.voters")}>a</TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label={t("electionScreen.tabs.publish")}>
                    <Publish />
                </TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label={t("electionScreen.tabs.logs")}>a</TabbedShowLayout.Tab>
            </TabbedShowLayout>
        </>
    )
}
