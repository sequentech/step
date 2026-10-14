// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
import {SettingsMessaging} from "@/resources/Settings/SettingsMessaging"
import {SettingsVotingChannels} from "@/resources/Settings/SettingsVotingChannel"
import {SettingsElectionsTypes} from "@/resources/Settings/SettingsElectionsTypes"
import {SettingsElectionsTypesCreate} from "@/resources/Settings/SettingsElectionsTypesCreate"
import {IPermissions} from "@/types/keycloak"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {Typography} from "@mui/material"
import {SettingsTrustees} from "@/resources/Settings/SettingsTrustees"
import {SettingsLookAndFeel} from "@/resources/Settings/SettingsLookAndFeel"
import {SettingsIntegrations} from "@/resources/Settings/SettingsIntegrations"
import {SettingsCountries} from "@/resources/Settings/SettingsCountries"
import SettingsLocalization from "@/resources/Settings/SettingsLocalization"
import {SettingsBackupRestore} from "@/resources/Settings/SettingsBackupRestore"
import {SettingsPreviews} from "@/resources/Settings/SettingsPreviews"
import {SettingsLogs} from "@/resources/Settings/SettingsLogs"

export const SettingsScreen: React.FC = () => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const canWriteTenant = authContext.isAuthorized(true, tenantId, IPermissions.TENANT_WRITE)
    // The tenant's log is read without changing any setting, as an auditor does.
    const canReadLogs = authContext.isAuthorized(true, tenantId, IPermissions.TENANT_LOGS_READ)
    const [open] = useSidebarState()

    const showSettingsMenu = authContext.isAuthorized(true, tenantId, IPermissions.SETTINGS_MENU)
    const showMessaging = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.MESSAGING_ACCOUNT_READ
    )

    if (!(canWriteTenant || canReadLogs) || !showSettingsMenu) {
        return (
            <ResourceListStyles.EmptyBox>
                <Typography variant="h4" paragraph>
                    {t("electionTypeScreen.noPermissions")}
                </Typography>
            </ResourceListStyles.EmptyBox>
        )
    }

    const settingsTabs = [
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
        ...(showMessaging
            ? [
                  {
                      label: t("messagingAccounts.tab"),
                      component: () => <SettingsMessaging />,
                  },
              ]
            : []),
        {
            label: t("electionTypeScreen.tabs.languages"),
            component: () => <Resource name="sequent_backend_tenant" list={SettingsLanguages} />,
        },
        {
            label: t("electionTypeScreen.tabs.localization"),
            component: () => <Resource name="sequent_backend_tenant" list={SettingsLocalization} />,
        },
        {
            label: t("electionTypeScreen.tabs.integrations"),
            component: () => <Resource name="sequent_backend_tenant" list={SettingsIntegrations} />,
        },
        {
            label: t("electionTypeScreen.tabs.lookAndFeel"),
            component: () => <Resource name="sequent_backend_tenant" list={SettingsLookAndFeel} />,
        },
        {
            label: t("electionTypeScreen.tabs.trustees"),
            component: () => <Resource name="sequent_backend_trustee" list={SettingsTrustees} />,
        },
        {
            label: "Countries",
            component: () => <Resource name="sequent_backend_tenant" list={SettingsCountries} />,
        },
        {
            label: t("electionTypeScreen.tabs.BackupRestore"),
            component: () => (
                <Resource name="sequent_backend_tenant" list={SettingsBackupRestore} />
            ),
        },
        {
            label: t("settings.previewScreen.label"),
            component: () => <Resource name="sequent_backend_preview" list={SettingsPreviews} />,
        },
    ]

    return (
        <Box
            sx={{maxWidth: `calc(100vw - ${open ? "352px" : "96px"})`, bgcolor: "background.paper"}}
            className="settings-box"
        >
            <HeaderTitle
                title={String(t("electionTypeScreen.common.settingTitle"))}
                subtitle={String(t("electionTypeScreen.common.settingSubtitle"))}
            />

            <Tabs
                elements={[
                    ...(canWriteTenant ? settingsTabs : []),
                    ...(canReadLogs
                        ? [{label: t("tenantLogs.tab"), component: () => <SettingsLogs />}]
                        : []),
                ]}
            />
        </Box>
    )
}
