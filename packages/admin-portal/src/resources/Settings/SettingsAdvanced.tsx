// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useState} from "react"
import {useTranslation} from "react-i18next"
import {SimpleForm, SaveButton, useNotify, useGetOne, BooleanInput, useUpdate} from "react-admin"
import Checkbox from "@mui/material/Checkbox"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {Divider, FormControlLabel, Typography} from "@mui/material"
import {EmptyBox} from "./SettingsElectionsTypes"
import {IMPORT_TENANT_CONFIG} from "@/queries/ImportTenantConfig"
import {EXPORT_TENANT_CONFIG} from "@/queries/ExportTenantConfig"
import {useMutation} from "@apollo/client"
import {ETasksExecution} from "@/types/tasksExecution"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {WidgetProps} from "@/components/Widget"
import {DownloadDocument} from "../User/DownloadDocument"
import {ImportDataDrawer} from "@/components/election-event/import-data/ImportDataDrawer"
import {ITenantSettings} from "@sequentech/ui-core"
import {Sequent_Backend_Tenant} from "@/gql/graphql"

interface ImportConfigurations {
    includeTenant?: boolean
    includeKeycloak?: boolean
    includeRoles?: boolean
}

export const SettingsAdvanced: React.FC<void> = () => {
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const {t} = useTranslation()
    const [isLoading, setLoading] = useState<boolean>(false)
    const [exportDocumentId, setExportDocumentId] = useState<string | undefined>()
    const [openImportDrawer, setOpenImportDrawer] = useState<boolean>(false)
    const [importConfigurations, setImportConfigurations] = useState<ImportConfigurations>({})
    const [export_tenant_config] = useMutation(EXPORT_TENANT_CONFIG, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.TENANT_READ,
            },
        },
    })

    // get tenant to retrieve the refresh menu setting for activate/deactivate polling
    const {data: tenant} = useGetOne<Sequent_Backend_Tenant>(
        "sequent_backend_tenant",
        {
            id: tenantId,
        },
        {
            enabled: !!tenantId,
        }
    )

    const [update] = useUpdate<Sequent_Backend_Tenant>()

    const defaultSettings: ITenantSettings = {has_refresh_menu: false}
    const settings: ITenantSettings = tenant?.settings ?? defaultSettings

    if (!tenantId) {
        return (
            <EmptyBox m={1}>
                <Typography variant="h4" paragraph>
                    {t("electionTypeScreen.common.emptyHeader")}
                </Typography>
            </EmptyBox>
        )
    }

    const handleUpdateTenantSettings = async (data: any) => {
        // setLoading(true)
        const settings = {
            ...tenant?.settings,
            has_refresh_menu: data.settings.has_refresh_menu,
        }
        update(
            "sequent_backend_tenant",
            {
                id: tenantId,
                data: {settings},
            },
            {
                onSuccess: () => {
                    // setLoading(false)
                },
                onError: (error: any) => {
                    // setLoading(false)
                    console.error("Error updating tenant settings", error)
                },
            }
        )
        // setLoading(false)
    }

    return (
        <>
            <Typography className="title" variant="h4">
                {t("settings.advanced.title")}
            </Typography>

            <StyledDivider />

            <SimpleForm
                className="backup-form"
                record={{
                    settings: {
                        has_refresh_menu: settings.has_refresh_menu,
                    },
                }}
                toolbar={
                    <SaveButton
                        className="save"
                        label={t("settings.advanced.saveTenantSettings.label")}
                        alwaysEnable
                        disabled={isLoading}
                    />
                }
                resource="sequent_backend_tenant"
                onSubmit={handleUpdateTenantSettings}
            >
                <BooleanInput
                    source="settings.has_refresh_menu"
                    label={t("settings.advanced.saveTenantSettings.value")}
                />
            </SimpleForm>
        </>
    )
}

const StyledDivider = () => {
    return <Divider sx={{padding: "10px"}} />
}
