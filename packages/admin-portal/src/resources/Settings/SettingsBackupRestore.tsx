// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"

import {useTranslation} from "react-i18next"
import {SimpleForm, TextInput, Toolbar, SaveButton, useNotify, useRecordContext} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IHelpLink, ITenantSettings, ITenantTheme} from "@sequentech/ui-core"
import {IPermissions} from "@/types/keycloak"
import {Typography} from "@mui/material"
import {EmptyBox} from "./SettingsElectionsTypes"
import {IMPORT_TENANT_CONFIG} from "@/queries/ImportTenantConfig"
import {EXPORT_TENANT_CONFIG} from "@/queries/ExportTenantConfig"
import {useMutation} from "@apollo/client"
import {ETasksExecution} from "@/types/tasksExecution"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {WidgetProps} from "@/components/Widget"
import {DownloadDocument} from "../User/DownloadDocument"

export const SettingsBackupRestore: React.FC<void> = () => {
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const {t} = useTranslation()
    const [exportDocumentId, setExportDocumentId] = useState<string | undefined>()
    const [isLoading, setLoading] = useState<boolean>(false)

    const [import_tenant_config] = useMutation(IMPORT_TENANT_CONFIG, {
        context: {
            // headers: {
            //     "x-hasura-role": IPermissions.ELECTION_EVENT_DELETE,
            // },
        },
    })
    const [export_tenant_config] = useMutation(EXPORT_TENANT_CONFIG, {
        context: {
            // headers: {
            //     "x-hasura-role": IPermissions.ELECTION_EVENT_DELETE,
            // },
        },
    })
    const handleBackup = async () => {
        const currWidget: WidgetProps = addWidget(ETasksExecution.EXPORT_TENANT_CONFIG)
        try {
            setLoading(true)
            let {data, errors} = await export_tenant_config({
                variables: {
                    tenantId,
                },
            })
            console.log({data})

            const documentId = data?.export_tenant_config?.document_id
            if (errors || !documentId) {
                updateWidgetFail(currWidget.identifier)
                console.log(`Error exporting users: ${errors}`)
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

    const handleRestore = async () => {
        //TODO: implement
        let {data, errors} = await import_tenant_config({
            variables: {
                tenantId,
                documentId: "",
            },
        })
        console.log({data})
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
            <SimpleForm
                className="backup-form"
                toolbar={
                    <SaveButton
                        className="save"
                        label={t("settings.backupRestore.backup.label")}
                        alwaysEnable
                        disabled={isLoading}
                    />
                }
                resource="sequent_backend_tenant"
                onSubmit={handleBackup}
            >
                <Typography className="title" variant="h4">
                    {t("settings.backupRestore.title")}
                </Typography>
                <Typography className="description" variant="body2">
                    {t("settings.backupRestore.backup.subtitle")}
                </Typography>
            </SimpleForm>

            <SimpleForm
                className="restore-form"
                toolbar={
                    <SaveButton
                        className="save"
                        label={t("settings.backupRestore.restore.label")}
                        alwaysEnable
                        disabled={isLoading}
                    />
                }
                resource="sequent_backend_tenant"
                onSubmit={handleRestore}
            >
                <Typography className="description" variant="body2">
                    {t("settings.backupRestore.restore.subtitle")}
                </Typography>
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
                        onSucess={() => setLoading(false)}
                    />
                </>
            )}
        </>
    )
}
