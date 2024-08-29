// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import {Box} from "@mui/system"
import {Resource} from "react-admin"
import {useTranslation} from "react-i18next"
import {AuthContext} from "@/providers/AuthContextProvider"
import {Tabs} from "@/components/Tabs"
import {HeaderTitle} from "@/components/HeaderTitle"
import {SettingsLanguages} from "@/resources/Settings/SettingsLanguages"
import {SettingsComunications} from "@/resources/Settings/SettingsComunications"
import {SettingsVotingChannels} from "@/resources/Settings/SettingsVotingChannel"
import {SettingsElectionsTypes} from "@/resources/Settings/SettingsElectionsTypes"
import {SettingsElectionsTypesCreate} from "@/resources/Settings/SettingsElectionsTypesCreate"
import {IPermissions} from "@/types/keycloak"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {Typography} from "@mui/material"
import {SettingsSchedules} from "@/resources/Settings/SettingsSchedules"
import {SettingsSchedulesCreate} from "@/resources/Settings/SettingsSchedulesCreate"
import {SettingsTrustees} from "@/resources/Settings/SettingsTrustees"
import {SettingsLookCustomization} from "@/resources/Settings/SettingsLookCustomization"

export const SettingsScreen: React.FC = () => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const hasPermissions = authContext.isAuthorized(true, tenantId, IPermissions.TENANT_WRITE)

    if (!hasPermissions) {
        return (
            <ResourceListStyles.EmptyBox>
                <Typography variant="h4" paragraph>
                    {t("electionTypeScreen.noPermissions")}
                </Typography>
            </ResourceListStyles.EmptyBox>
        )
    }

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
                    {
                        label: t("electionTypeScreen.tabs.lookCustomization"),
                        component: () => (
                            <Resource
                                name="sequent_backend_tenant"
                                list={SettingsLookCustomization}
                            />
                        ),
                    },
                    {
                        label: t("electionTypeScreen.tabs.schedules"),
                        component: () => (
                            <Resource
                                name="sequent_backend_tenant"
                                list={SettingsSchedules}
                                create={SettingsSchedulesCreate}
                                edit={SettingsSchedulesCreate}
                                show={SettingsSchedulesCreate}
                            />
                        ),
                    },
                    {
                        label: t("electionTypeScreen.tabs.trustees"),
                        component: () => (
                            <Resource name="sequent_backend_trustee" list={SettingsTrustees} />
                        ),
                    },
                ]}
            />
        </Box>
    )
}
