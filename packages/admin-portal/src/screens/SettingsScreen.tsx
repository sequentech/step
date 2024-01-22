import React, { useEffect } from "react"
import {Box} from "@mui/system"
import {Resource} from "react-admin"
import {useTranslation} from "react-i18next"

import {Tabs} from "../components/Tabs"
import {HeaderTitle} from "../components/HeaderTitle"
import {SettingsLanguages} from "../resources/Settings/SettingsLanguages"
import {SettingsComunications} from "../resources/Settings/SettingsComunications"
import {SettingsVotingChannels} from "../resources/Settings/SettingsVotingChannel"
import {SettingsElectionsTypes} from "../resources/Settings/SettingsElectionsTypes"
import {SettingsElectionsTypesCreate} from "@/resources/Settings/SettingsElectionsTypesCreate"

export const SettingsScreen: React.FC = () => {
    const {t, i18n} = useTranslation()

    useEffect(() => {
        const dir = i18n.dir(i18n.language)
        document.documentElement.dir = dir
    }, [i18n, i18n.language])

    return (
        <Box>
            <HeaderTitle
                title={t("electionTypeScreen.common.settingTitle")}
                subtitle={t("electionTypeScreen.common.settingSubtitle")}
            />

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
                    {
                        label: t("electionTypeScreen.tabs.communications"),
                        component: () => (
                            <Resource name="sequent_backend_tenant" list={SettingsComunications} />
                        ),
                    },
                    {
                        label: t("electionTypeScreen.tabs.languages"),
                        component: () => (
                            <Resource name="sequent_backend_tenant" list={SettingsLanguages} />
                        ),
                    },
                ]}
            />
        </Box>
    )
}
