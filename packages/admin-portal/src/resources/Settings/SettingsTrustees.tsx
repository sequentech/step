// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useContext, useEffect} from "react"

import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import {faPlus} from "@fortawesome/free-solid-svg-icons"

import {Box, Button, Drawer, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"

import {List, TextField, TextInput, useDelete, Identifier, DatagridConfigurable} from "react-admin"

import {Dialog} from "@sequentech/ui-essentials"
import {IconButton} from "@sequentech/ui-essentials"
import {ListActions} from "@/components/ListActions"
import {ActionsColumn} from "@/components/ActionButons"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

import {SettingsTrusteesCreate} from "./SettingsTrusteesCreate"
import {SettingsTrusteesEdit} from "./SettingsTrusteesEdit"
import {PasswordDialog} from "@/components/election-event/export-data/PasswordDialog"
import {useMutation} from "@apollo/client"
import {EXPORT_TRUSTEES} from "@/queries/ExportTrustees"
import {ExportTrusteesMutation} from "@/gql/graphql"
import {generateRandomPassword} from "@/services/Password"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {WidgetProps} from "@/components/Widget"
import {ETasksExecution} from "@/types/tasksExecution"
import {FormStyles} from "@/components/styles/FormStyles"
import {DownloadDocument} from "../User/DownloadDocument"

const EmptyBox = styled(Box)`
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    width: 100%;
`

const useActionPermissions = () => {
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    const canCreateTrustee = authContext.isAuthorized(true, tenantId, IPermissions.TRUSTEE_WRITE)
    const canReadTrustee = authContext.isAuthorized(true, tenantId, IPermissions.TRUSTEE_READ)
    const canExportTrustees = authContext.isAuthorized(true, tenantId, IPermissions.TRUSTEES_EXPORT)

    return {
        canCreateTrustee,
        canReadTrustee,
        canExportTrustees,
    }
}

const OMIT_FIELDS = ["id", "public_key"]
const Filters: Array<ReactElement> = [
    <TextInput label="Name" source="name" key={0} />,
    <TextInput label="Public Key" source="public_key" key={0} />,
]

export const SettingsTrustees: React.FC<void> = () => {
    const {t} = useTranslation()
    const [deleteOne] = useDelete()
    const {canCreateTrustee, canReadTrustee, canExportTrustees} = useActionPermissions()
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()

    const [open, setOpen] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [deleteId, setDeleteId] = React.useState<Identifier | undefined>()
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)
    const [recordId, setRecordId] = React.useState<Identifier | undefined>(undefined)
    const [loadingExport, setLoadingExport] = React.useState<boolean>(false)
    const [exportDocumentId, setExportDocumentId] = React.useState<string | null>(null)

    const [openPasswordDialog, setOpenPasswordDialog] = React.useState(false)
    const [password, setPassword] = React.useState<string | null>(null)

    const [exportTrustees] = useMutation<ExportTrusteesMutation>(EXPORT_TRUSTEES, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.TRUSTEES_EXPORT,
            },
        },
    })

    useEffect(() => {
        if (recordId) {
            setOpen(true)
        }
    }, [recordId])

    const handleCloseCreateDrawer = () => {
        setRecordId(undefined)
        setOpenDrawer(false)
        setOpen(false)
    }

    const handleOpenCreateDrawer = () => {
        setRecordId(undefined)
        setOpenDrawer(true)
        setOpen(false)
    }

    const handleCloseEditDrawer = () => {
        setOpen(false)
        setTimeout(() => {
            setRecordId(undefined)
        }, 400)
    }

    const editAction = (id: Identifier) => {
        setRecordId(id)
    }

    const deleteAction = (id: Identifier) => {
        setOpenDeleteModal(true)
        setDeleteId(id)
    }

    const confirmDeleteAction = () => {
        deleteOne("sequent_backend_trustee", {id: deleteId})
        setDeleteId(undefined)
    }

    const actions: any[] = [
        {icon: <EditIcon />, action: editAction},
        {icon: <DeleteIcon />, action: deleteAction},
    ]

    const CreateButton = () => (
        <Button onClick={handleOpenCreateDrawer}>
            <IconButton icon={faPlus} fontSize="24px" />
            {t("trusteesSettingsScreen.common.createNew")}
        </Button>
    )

    const Empty = () => (
        <EmptyBox m={1}>
            <Typography variant="h4" paragraph>
                {t("trusteesSettingsScreen.common.emptyHeader")}
            </Typography>
            {canCreateTrustee ? (
                <>
                    <Typography variant="body1" paragraph>
                        {t("electionTypeScreen.common.emptyBody")}
                    </Typography>
                    <CreateButton />
                </>
            ) : null}
        </EmptyBox>
    )

    if (!canReadTrustee) {
        return <Empty />
    }

    const resetState = () => {
        setOpenPasswordDialog(false)
        setPassword(null)
        setLoadingExport(false)
        setExportDocumentId(null)
    }

    const doExport = async () => {
        const currWidget: WidgetProps = addWidget(ETasksExecution.EXPORT_TRUSTEES, undefined)
        setLoadingExport(true)
        const generatedPassword = generateRandomPassword()
        setPassword(generatedPassword)

        try {
            const {data: exportTrusteesData, errors} = await exportTrustees({
                variables: {
                    password: generatedPassword,
                },
            })

            const documentId = exportTrusteesData?.exportTrustees?.document_id
            if (errors || !documentId) {
                updateWidgetFail(currWidget.identifier)
                console.log(`Error exporting users: ${errors}`)
                resetState()
                return
            }

            const task_id = exportTrusteesData?.exportTrustees?.task_execution.id
            setWidgetTaskId(currWidget.identifier, task_id)
            setExportDocumentId(documentId)
            setOpenPasswordDialog(true)
        } catch (error) {
            console.log(error)
            updateWidgetFail(currWidget.identifier)
            resetState()
        }
    }

    return (
        <>
            <List
                filters={Filters}
                actions={
                    <ListActions
                        custom
                        withFilter
                        open={openDrawer}
                        setOpen={setOpenDrawer}
                        isExportDisabled={openPasswordDialog || loadingExport}
                        Component={<SettingsTrusteesCreate close={handleCloseCreateDrawer} />}
                        withComponent={canCreateTrustee}
                        withExport={canExportTrustees}
                        doExport={doExport}
                    />
                }
                empty={<Empty />}
            >
                <DatagridConfigurable omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="public_key" />
                    <TextField source="name" />

                    <ActionsColumn actions={actions} />
                </DatagridConfigurable>
            </List>

            <Drawer
                anchor="right"
                open={openDrawer}
                onClose={handleCloseCreateDrawer}
                PaperProps={{
                    sx: {width: "40%"},
                }}
            >
                <SettingsTrusteesCreate close={handleCloseCreateDrawer} />
            </Drawer>

            <Drawer
                anchor="right"
                open={open}
                onClose={handleCloseEditDrawer}
                PaperProps={{
                    sx: {width: "40%"},
                }}
            >
                <SettingsTrusteesEdit id={recordId} close={handleCloseEditDrawer} />
            </Drawer>

            <Dialog
                variant="warning"
                open={openDeleteModal}
                ok={t("common.label.delete")}
                cancel={t("common.label.cancel")}
                title={t("common.label.warning")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmDeleteAction()
                    }
                    setOpenDeleteModal(false)
                }}
            >
                {t("common.message.delete")}
            </Dialog>
            {exportDocumentId && (
                <>
                    <FormStyles.ShowProgress />
                    <DownloadDocument
                        documentId={exportDocumentId}
                        fileName={`trustees-export.ezip`}
                        onDownload={() => {
                            setExportDocumentId(null)
                        }}
                        onSuccess={() => undefined}
                    />
                </>
            )}
            {openPasswordDialog && password && (
                <PasswordDialog password={password} onClose={resetState} />
            )}
        </>
    )
}
