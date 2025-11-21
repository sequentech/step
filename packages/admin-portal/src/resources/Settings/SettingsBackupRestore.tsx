// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useState} from "react"
import {useTranslation} from "react-i18next"
import {SimpleForm, SaveButton, useNotify} from "react-admin"
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

interface ImportConfigurations {
    includeTenant?: boolean
    includeKeycloak?: boolean
    includeRoles?: boolean
}

export const SettingsBackupRestore: React.FC<void> = () => {
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

    const handleBackup = async () => {
        const currWidget: WidgetProps = addWidget(ETasksExecution.EXPORT_TENANT_CONFIG, undefined)
        try {
            setLoading(true)
            let {data, errors} = await export_tenant_config({
                variables: {
                    tenantId,
                },
            })

            const documentId = data?.export_tenant_config?.document_id
            if (errors || !documentId) {
                updateWidgetFail(currWidget.identifier)
                console.log(`Error exporting tenant config: ${errors}`)
                setLoading(false)
                return
            }

            const task_id = data?.export_tenant_config?.task_execution.id
            setWidgetTaskId(currWidget.identifier, task_id)
            setExportDocumentId(documentId)
        } catch (e) {
            updateWidgetFail(currWidget.identifier)
            setLoading(false)
        }
    }

    const [import_tenant_config] = useMutation(IMPORT_TENANT_CONFIG, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.TENANT_WRITE,
            },
        },
    })

    const handleRestore = async (documentId: string, sha256: string) => {
        const currWidget: WidgetProps = addWidget(ETasksExecution.IMPORT_TENANT_CONFIG, undefined)
        try {
            setLoading(true)
            setOpenImportDrawer(false)
            let {data, errors} = await import_tenant_config({
                variables: {
                    tenantId,
                    documentId,
                    importConfigurations: {
                        include_tenant: importConfigurations?.includeTenant,
                        include_keycloak: importConfigurations?.includeKeycloak,
                        include_roles: importConfigurations?.includeRoles,
                    },
                    sha256,
                },
            })

            if (errors) {
                updateWidgetFail(currWidget.identifier)
                console.log(`Error importing tenant config: ${errors}`)
                setLoading(false)
                return
            }

            const task_id = data?.import_tenant_config?.task_execution.id
            setWidgetTaskId(currWidget.identifier, task_id)
        } catch (e) {
            updateWidgetFail(currWidget.identifier)
            setLoading(false)
        }
    }

    const handleImportOptionsChange = (
        name: string,
        event: React.ChangeEvent<HTMLInputElement>
    ) => {
        const {checked} = event.target
        setImportConfigurations((prev) => ({
            ...prev,
            [name]: checked,
        }))
    }

    if (!tenantId) {
        return (
            <EmptyBox m={1}>
                <Typography variant="h4" paragraph>
                    {t("electionTypeScreen.common.emptyHeader")}
                </Typography>
            </EmptyBox>
        )
    }

    return (
        <>
            <Typography className="title" variant="h4">
                {t("settings.backupRestore.title")}
            </Typography>

            <StyledDivider />

            <SimpleForm
                className="backup-form"
                toolbar={
                    <SaveButton
                        className="save"
                        label={String(t("settings.backupRestore.backup.label"))}
                        alwaysEnable
                        disabled={isLoading}
                    />
                }
                resource="sequent_backend_tenant"
                onSubmit={handleBackup}
            >
                <Typography variant="h6">{t("settings.backupRestore.backup.subtitle")}</Typography>
            </SimpleForm>

            <StyledDivider />

            <SimpleForm
                className="restore-form"
                resource="sequent_backend_tenant"
                toolbar={
                    <SaveButton
                        className="save"
                        label={String(t("settings.backupRestore.restore.label"))}
                        alwaysEnable
                        disabled={
                            // TODO: fix disable mode
                            isLoading ||
                            !(
                                !!importConfigurations?.includeTenant &&
                                !!importConfigurations?.includeKeycloak &&
                                !!importConfigurations?.includeRoles
                            )
                        }
                    />
                }
                onSubmit={() => {
                    setOpenImportDrawer(true)
                }}
            >
                <Typography variant="h6">{t("settings.backupRestore.restore.subtitle")}</Typography>

                <FormControlLabel
                    control={
                        <Checkbox
                            checked={importConfigurations?.includeTenant}
                            onChange={(event) => handleImportOptionsChange("includeTenant", event)}
                        />
                    }
                    label={String(t("settings.backupRestore.restore.tenantConfigOption"))}
                />

                <FormControlLabel
                    control={
                        <Checkbox
                            checked={importConfigurations?.includeKeycloak}
                            onChange={(event) =>
                                handleImportOptionsChange("includeKeycloak", event)
                            }
                        />
                    }
                    label={String(t("settings.backupRestore.restore.keycloakConfigOption"))}
                />

                <FormControlLabel
                    control={
                        <Checkbox
                            checked={importConfigurations?.includeRoles}
                            onChange={(event) => handleImportOptionsChange("includeRoles", event)}
                        />
                    }
                    label={String(t("settings.backupRestore.restore.RolesConfigOption"))}
                />
            </SimpleForm>

            {exportDocumentId && (
                <>
                    <DownloadDocument
                        documentId={exportDocumentId}
                        fileName={`tenant-config-${tenantId}-export.zip`}
                        onDownload={() => {
                            console.log("onDownload called")
                            setExportDocumentId(undefined)
                            setLoading(false)
                        }}
                        onSuccess={() => setLoading(false)}
                    />
                </>
            )}

            <ImportDataDrawer
                open={openImportDrawer}
                closeDrawer={() => setOpenImportDrawer(false)}
                title="settings.backupRestore.restore.title"
                subtitle="settings.backupRestore.restore.subtitle"
                paragraph="settings.backupRestore.restore.paragraph"
                doImport={handleRestore}
                errors={null}
            />
        </>
    )
}

const StyledDivider = () => {
    return <Divider sx={{padding: "10px"}} />
}
