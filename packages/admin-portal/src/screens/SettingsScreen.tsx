// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import {Box} from "@mui/system"
import {Resource, useSidebarState} from "react-admin"
import {useTranslation} from "react-i18next"
import {AuthContext} from "@/providers/AuthContextProvider"
import {Tabs} from "@/components/Tabs"
import {HeaderTitle} from "@/components/HeaderTitle"
import {SettingsLanguages} from "@/resources/Settings/SettingsLanguages"
import {SettingsTemplates} from "@/resources/Settings/SettingsTemplates"
import {SettingsVotingChannels} from "@/resources/Settings/SettingsVotingChannel"
import {SettingsElectionsTypes} from "@/resources/Settings/SettingsElectionsTypes"
import {SettingsElectionsTypesCreate} from "@/resources/Settings/SettingsElectionsTypesCreate"
import {IPermissions} from "@/types/keycloak"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {Typography} from "@mui/material"
import {SettingsTrustees} from "@/resources/Settings/SettingsTrustees"
import {SettingsLookAndFeel} from "@/resources/Settings/SettingsLookAndFeel"
import {SettingsCountries} from "@/resources/Settings/SettingsCountries"
import SettingsLocalization from "@/resources/Settings/SettingsLocalization"
import {SettingsBackupRestore} from "@/resources/Settings/SettingsBackupRestore"
import {SettingsAuthentication} from "@/resources/Settings/SettingsAuthentication"

export const SettingsScreen: React.FC = () => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const hasPermissions = authContext.isAuthorized(true, tenantId, IPermissions.TENANT_WRITE)
    const [open] = useSidebarState()

    const showSettingsMenu = authContext.isAuthorized(true, tenantId, IPermissions.SETTINGS_MENU)

    if (!hasPermissions || !showSettingsMenu) {
        return (
            <ResourceListStyles.EmptyBox>
                <Typography variant="h4" paragraph>
                    {t("electionTypeScreen.noPermissions")}
                </Typography>
            </ResourceListStyles.EmptyBox>
        )
    }

    return (
        <Box
            sx={{maxWidth: `calc(100vw - ${open ? "352px" : "96px"})`, bgcolor: "background.paper"}}
            className="settings-box"
        >
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
                        label: t("electionTypeScreen.tabs.templates"),
                        component: () => (
                            <Resource name="sequent_backend_tenant" list={SettingsTemplates} />
                        ),
                    },
                    {
                        label: t("electionTypeScreen.tabs.languages"),
                        component: () => (
                            <Resource name="sequent_backend_tenant" list={SettingsLanguages} />
                        ),
                    },
                    {
                        label: t("electionTypeScreen.tabs.localization"),
                        component: () => (
                            <Resource name="sequent_backend_tenant" list={SettingsLocalization} />
                        ),
                    },
                    {
                        label: t("electionTypeScreen.tabs.lookAndFeel"),
                        component: () => (
                            <Resource name="sequent_backend_tenant" list={SettingsLookAndFeel} />
                        ),
                    },
                    {
                        label: t("electionTypeScreen.tabs.trustees"),
                        component: () => (
                            <Resource name="sequent_backend_trustee" list={SettingsTrustees} />
                        ),
                    },
                    {
                        label: "Countries",
                        component: () => (
                            <Resource name="sequent_backend_tenant" list={SettingsCountries} />
                        ),
                    },
                    {
                        label: "Authentication",
                        component: () => (
                            <Resource name="sequent_backend_tenant" list={SettingsAuthentication} />
                        ),
                    },
                    {
                        label: t("electionTypeScreen.tabs.BackupRestore"),
                        component: () => (
                            <Resource name="sequent_backend_tenant" list={SettingsBackupRestore} />
                        ),
                    },
                ]}
            />
        </Box>
    )
}
