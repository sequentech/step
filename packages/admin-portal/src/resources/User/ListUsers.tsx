// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023, 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {ReactElement, useContext} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    TextInput,
    BooleanField,
    Identifier,
    WrapperField,
    useRefresh,
    useNotify,
    useGetList,
    FunctionField,
    Button as ReactAdminButton,
} from "react-admin"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {useTenantStore} from "@/providers/TenantContextProvider"
import UploadIcon from "@mui/icons-material/Upload"
import {ListActions} from "@/components/ListActions"
import {Button, Chip, Drawer, Typography} from "@mui/material"
import {Dialog} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"
import {Action, ActionsColumn} from "@/components/ActionButons"
import EditIcon from "@mui/icons-material/Edit"
import MailIcon from "@mui/icons-material/Mail"
import DeleteIcon from "@mui/icons-material/Delete"
import {EditUser} from "./EditUser"
import {AudienceSelection, SendCommunication} from "./SendCommunication"
import {CreateUser} from "./CreateUser"
import {AuthContext} from "@/providers/AuthContextProvider"
import {DeleteUserMutation, ExportUsersMutation} from "@/gql/graphql"
import {DELETE_USER} from "@/queries/DeleteUser"
import {useMutation, useQuery} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {IRole, IUser} from "sequent-core"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {ImportVotersTabs} from "@/components/election-event/import-data/ImportVotersTabs"
import importDrawerState from "@/atoms/import-drawer-state"
import {useAtom} from "jotai"
import {FormStyles} from "@/components/styles/FormStyles"
import {EXPORT_USERS} from "@/queries/ExportUsers"
import {DownloadDocument} from "./DownloadDocument"

const OMIT_FIELDS: Array<string> = ["id", "email_verified"]

const Filters: Array<ReactElement> = [
    <TextInput key="email" source="email" />,
    <TextInput key="first_name" source="first_name" />,
    <TextInput key="last_name" source="last_name" />,
    <TextInput key="username" source="username" />,
]

export interface ListUsersProps {
    aside?: ReactElement
    electionEventId?: string
    electionId?: string
}

export const ListUsers: React.FC<ListUsersProps> = ({aside, electionEventId, electionId}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)

    const [open, setOpen] = React.useState(false)
    const [openImport, setOpenImport] = useAtom(importDrawerState)
    const [openExport, setOpenExport] = React.useState(false)
    const [exporting, setExporting] = React.useState(false)
    const [exportDocumentId, setExportDocumentId] = React.useState<string | undefined>()
    const [openNew, setOpenNew] = React.useState(false)
    const [audienceSelection, setAudienceSelection] = React.useState<AudienceSelection>(
        AudienceSelection.SELECTED
    )
    const [openSendCommunication, setOpenSendCommunication] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [openDeleteBulkModal, setOpenDeleteBulkModal] = React.useState(false)
    const [selectedIds, setSelectedIds] = React.useState<Identifier[]>([])
    const [deleteId, setDeleteId] = React.useState<string | undefined>()
    const [openDrawer, setOpenDrawer] = React.useState(false)
    const [recordIds, setRecordIds] = React.useState<Array<Identifier>>([])
    const authContext = useContext(AuthContext)
    const refresh = useRefresh()
    const [deleteUser] = useMutation<DeleteUserMutation>(DELETE_USER)
    const [deleteUsers] = useMutation<DeleteUserMutation>(DELETE_USER)
    const [exportUsers] = useMutation<ExportUsersMutation>(EXPORT_USERS)
    const notify = useNotify()
    const {data: rolesList} = useGetList<IRole & {id: string}>("role", {
        pagination: {page: 1, perPage: 9999},
        sort: {field: "last_updated_at", order: "DESC"},
        filter: {
            tenant_id: tenantId,
        },
    })
    const canEditUsers = authContext.isAuthorized(true, tenantId, IPermissions.VOTER_WRITE)
    const canSendCommunications = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.NOTIFICATION_SEND
    )

    const handleClose = () => {
        setRecordIds([])
        setOpenSendCommunication(false)
        setOpenDeleteModal(false)
        setOpenDeleteBulkModal(false)
        setOpenDrawer(false)
        setOpenNew(false)
        setOpen(false)
    }

    const editAction = (id: Identifier) => {
        setOpen(true)
        setOpenNew(false)
        setOpenDeleteModal(false)
        setOpenDeleteBulkModal(false)
        setOpenSendCommunication(false)
        setRecordIds([id as string])
    }

    const sendCommunicationForIdAction = (id: Identifier) => {
        sendCommunicationAction([id])
    }

    const sendCommunicationAction = (
        ids: Array<Identifier>,
        audienceSelection = AudienceSelection.SELECTED
    ) => {
        setOpen(false)
        setOpenNew(false)
        setOpenDeleteModal(false)
        setOpenDeleteBulkModal(false)
        setOpenSendCommunication(true)

        setAudienceSelection(audienceSelection)
        setRecordIds(ids)
    }

    const deleteAction = (id: Identifier) => {
        if (!electionEventId && authContext.userId === id) {
            return
        }
        setOpen(false)
        setOpenNew(false)
        setOpenSendCommunication(false)
        setOpenDeleteBulkModal(false)
        setOpenDeleteModal(true)
        setDeleteId(id as string)
    }

    const confirmDeleteAction = async () => {
        const {errors} = await deleteUser({
            variables: {
                tenantId: tenantId,
                electionEventId: electionEventId,
                userId: deleteId,
            },
        })
        if (errors) {
            notify(
                t(
                    `usersAndRolesScreen.${
                        electionEventId ? "voters" : "users"
                    }.notifications.deleteError`
                ),
                {type: "error"}
            )
            console.log(`Error deleting user: ${errors}`)
            return
        }
        notify(
            t(
                `usersAndRolesScreen.${
                    electionEventId ? "voters" : "users"
                }.notifications.deleteSuccess`
            ),
            {type: "success"}
        )
        setDeleteId(undefined)
        refresh()
    }

    const actions: Action[] = [
        {
            icon: <MailIcon />,
            action: sendCommunicationForIdAction,
            showAction: () => canSendCommunications,
        },
        {
            icon: <EditIcon />,
            action: editAction,
            showAction: () => canEditUsers,
        },
        {
            icon: <DeleteIcon />,
            action: deleteAction,
            showAction: () => canEditUsers,
        },
    ]

    async function confirmDeleteBulkAction() {
        const {errors} = await deleteUsers({
            variables: {
                tenantId: tenantId,
                electionEventId: electionEventId,
                userId: selectedIds,
            },
        })

        if (errors) {
            notify(
                t(
                    `usersAndRolesScreen.${
                        electionEventId ? "voters" : "users"
                    }.notifications.deleteError`
                ),
                {type: "error"}
            )
            return
        }

        notify(
            t(
                `usersAndRolesScreen.${
                    electionEventId ? "voters" : "users"
                }.notifications.deleteSuccess`
            ),
            {type: "success"}
        )

        refresh()
    }

    // @ts-ignore
    function BulkActions(props) {
        return (
            <>
                {canSendCommunications && (
                    <Button
                        variant="actionbar"
                        key="send-notification"
                        onClick={() => {
                            sendCommunicationAction(
                                props.selectedIds ?? [],
                                AudienceSelection.SELECTED
                            )
                        }}
                    >
                        <ResourceListStyles.MailIcon />
                        {t(`sendCommunication.send`)}
                    </Button>
                )}

                {canEditUsers && (
                    <Button
                        variant="actionbar"
                        onClick={() => {
                            setSelectedIds(props.selectedIds)
                            setOpenDeleteBulkModal(true)
                        }}
                    >
                        <ResourceListStyles.DeleteIcon />
                        {t("common.label.delete")}
                    </Button>
                )}
            </>
        )
    }

    const handleImport = () => {
        setOpenImport(true)
    }

    const handleExport = () => {
        setExporting(false)
        setExportDocumentId(undefined)
        setOpenExport(true)
    }

    const confirmExportAction = async () => {
        setExportDocumentId(undefined)
        setExporting(true)
        const {data: exportUsersData, errors} = await exportUsers({
            variables: {
                tenantId: tenantId,
                electionEventId: electionEventId,
                electionId: electionId,
            },
        })
        if (errors || !exportUsersData) {
            setExporting(false)
            setOpenExport(false)
            notify(
                t(
                    `usersAndRolesScreen.${
                        electionEventId ? "voters" : "users"
                    }.notifications.exportError`
                ),
                {type: "error"}
            )
            console.log(`Error exporting users: ${errors}`)
            return
        }
        let documentId = exportUsersData.export_users?.document_id
        setExportDocumentId(documentId)
    }

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t(`usersAndRolesScreen.${electionEventId ? "voters" : "users"}.emptyHeader`)}
            </Typography>
            {canEditUsers ? (
                <>
                    <Typography variant="body1" paragraph>
                        {t(`usersAndRolesScreen.${electionEventId ? "voters" : "users"}.askCreate`)}
                    </Typography>
                    <ResourceListStyles.EmptyButtonList>
                        <Button onClick={() => setOpenNew(true)}>
                            <ResourceListStyles.CreateIcon icon={faPlus} />
                            {t(
                                `usersAndRolesScreen.${
                                    electionEventId ? "voters" : "users"
                                }.create.subtitle`
                            )}
                        </Button>
                        <ReactAdminButton onClick={handleImport} label={t("common.label.import")}>
                            <UploadIcon />
                        </ReactAdminButton>
                    </ResourceListStyles.EmptyButtonList>
                </>
            ) : null}
        </ResourceListStyles.EmptyBox>
    )

    return (
        <>
            <List
                resource="user"
                queryOptions={{
                    refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
                }}
                empty={<Empty />}
                actions={
                    <ListActions
                        withImport
                        doImport={handleImport}
                        withExport
                        doExport={handleExport}
                        open={openDrawer}
                        setOpen={setOpenDrawer}
                        Component={
                            <CreateUser electionEventId={electionEventId} close={handleClose} />
                        }
                        extraActions={[
                            <Button
                                key="send-notification"
                                onClick={() => {
                                    sendCommunicationAction([], AudienceSelection.ALL_USERS)
                                }}
                            >
                                <ResourceListStyles.MailIcon />
                                {t("sendCommunication.send")}
                            </Button>,
                        ]}
                    />
                }
                filter={{
                    tenant_id: tenantId,
                    election_event_id: electionEventId,
                    election_id: electionId,
                }}
                aside={aside}
                filters={Filters}
            >
                <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={<BulkActions />}>
                    <TextField source="id" />
                    <TextField source="email" />
                    <BooleanField source="email_verified" />
                    <BooleanField source="enabled" />
                    <TextField source="first_name" />
                    <TextField
                        label={t("usersAndRolesScreen.common.mobileNumber")}
                        source="attributes['sequent.read-only.mobile-number']"
                    />
                    <TextField source="last_name" />
                    <TextField source="username" />
                    {electionEventId && (
                        <FunctionField
                            label={t("usersAndRolesScreen.users.fields.area")}
                            render={(record: IUser) =>
                                record?.area?.name ? <Chip label={record?.area?.name ?? ""} /> : "-"
                            }
                        />
                    )}
                    {electionEventId && (
                        <FunctionField
                            source="has_voted"
                            label={t("usersAndRolesScreen.users.fields.has_voted")}
                            render={(record: IUser, source: string | undefined) => {
                                let newRecord = {
                                    has_voted: (record?.votes_info?.length ?? 0) > 0,
                                    ...record,
                                }
                                return <BooleanField record={newRecord} source={source} />
                            }}
                        />
                    )}
                    <WrapperField source="actions" label="Actions">
                        <ActionsColumn actions={actions} />
                    </WrapperField>
                </DatagridConfigurable>
            </List>
            <ResourceListStyles.Drawer anchor="right" open={open} onClose={handleClose}>
                <EditUser
                    id={recordIds[0] as string}
                    electionEventId={electionEventId}
                    close={handleClose}
                    rolesList={rolesList || []}
                />
            </ResourceListStyles.Drawer>
            <ResourceListStyles.Drawer
                anchor="right"
                open={openSendCommunication}
                onClose={handleClose}
            >
                <SendCommunication
                    ids={recordIds}
                    audienceSelection={audienceSelection}
                    electionEventId={electionEventId}
                    close={handleClose}
                />
            </ResourceListStyles.Drawer>
            <ResourceListStyles.Drawer anchor="right" open={openNew} onClose={handleClose}>
                <CreateUser electionEventId={electionEventId} close={handleClose} />
            </ResourceListStyles.Drawer>
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
                {t(`usersAndRolesScreen.${electionEventId ? "voters" : "users"}.delete.body`)}
            </Dialog>

            <Drawer
                anchor="right"
                open={openImport}
                onClose={() => {
                    setOpenImport(false)
                }}
                PaperProps={{
                    sx: {width: "30%"},
                }}
            >
                <ImportVotersTabs doRefresh={() => refresh()} />
            </Drawer>

            <Dialog
                variant="warning"
                open={openDeleteBulkModal}
                ok={t("common.label.delete")}
                cancel={t("common.label.cancel")}
                title={t("common.label.warning")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmDeleteBulkAction()
                    }
                    setOpenDeleteBulkModal(false)
                }}
            >
                {t(`usersAndRolesScreen.${electionEventId ? "voters" : "users"}.delete.bulkBody`)}
            </Dialog>

            <Dialog
                variant="info"
                open={openExport}
                ok={t("common.label.export")}
                okEnabled={() => !exporting}
                cancel={t("common.label.cancel")}
                title={t("common.label.export")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmExportAction()
                    } else {
                        setExportDocumentId(undefined)
                        setExporting(false)
                        setOpenExport(false)
                    }
                }}
            >
                {t("common.export")}
                <FormStyles.ReservedProgressSpace>
                    {exporting ? <FormStyles.ShowProgress /> : null}
                    {exporting && exportDocumentId ? (
                        <DownloadDocument
                            documentId={exportDocumentId}
                            electionEventId={electionEventId ?? ""}
                            fileName={`users-export.tsv`}
                            onDownload={() => {
                                console.log("onDownload called")
                                setExportDocumentId(undefined)
                                setExporting(false)
                                setOpenExport(false)
                            }}
                        />
                    ) : null}
                </FormStyles.ReservedProgressSpace>
            </Dialog>
        </>
    )
}
