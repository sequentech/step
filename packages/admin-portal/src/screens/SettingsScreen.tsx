import React from "react"
import {Box} from "@mui/system"
import {Resource} from "react-admin"

import {Tabs} from "../components/Tabs"
import {HeaderTitle} from "../components/HeaderTitle"
import {SettingsElectionsTypes} from "../resources/Settings/SettingsElectionsTypes"
import {SettingsElectionsTypesCreate} from "@/resources/Settings/SettingsElectionsTypesCreate"

import {SettingsVotingChannels} from "../resources/Settings/SettingsVotingChannel"
import {useTranslation} from "react-i18next"

export const SettingsScreen: React.FC = () => {
    const {t} = useTranslation()

    return (
        <Box>
            <HeaderTitle title={t("electionTypeScreen.common.title")} subtitle={t("electionTypeScreen.tabs.electionTypes")} />

            <Tabs
                elements={[
                    {
                        label: t("electionTypeScreen.tabs.electionTypes"),
                        component: () => (
                            <Resource
                                name="sequent_backend_election_type"
                                list={SettingsElectionsTypes}
                                create={SettingsElectionsTypesCreate}
                                edit={SettingsElectionsTypesCreate}
                                show={SettingsElectionsTypesCreate}
                            />
                        ),
                    },
                    {
                        label: t("electionTypeScreen.tabs.votingChannels"),
                        component: () => (
                            <Resource name="sequent_backend_tenant" list={SettingsVotingChannels} />
                        ),
                    },
                    {label: t("electionTypeScreen.tabs.communications"), component: () => <></>},
                    {label: t("electionTypeScreen.tabs.languages"), component: () => <></>},
                ]}
            />
        </Box>
    )
}
