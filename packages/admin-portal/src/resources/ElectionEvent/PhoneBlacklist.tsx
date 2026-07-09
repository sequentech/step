// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useState} from "react"
import {
    DatagridConfigurable,
    DateInput,
    List,
    RaRecord,
    TextField,
    TextInput,
    useNotify,
    useRecordContext,
    useRefresh,
} from "react-admin"
import {useMutation} from "@apollo/client"
import {
    Alert,
    Box,
    Button,
    Drawer,
    IconButton,
    TextField as MuiTextField,
    Typography,
} from "@mui/material"
import DeleteIcon from "@mui/icons-material/Delete"
import EditIcon from "@mui/icons-material/Edit"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {useTranslation} from "react-i18next"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {DrawerStyles} from "@/components/styles/DrawerStyles"
import {ListActions} from "@/components/ListActions"
import ElectionHeader from "@/components/ElectionHeader"
import {CREATE_PHONE_BLACKLIST_ENTRY} from "@/queries/CreatePhoneBlacklistEntry"
import {UPDATE_PHONE_BLACKLIST_ENTRY} from "@/queries/UpdatePhoneBlacklistEntry"
import {DELETE_PHONE_BLACKLIST_ENTRY} from "@/queries/DeletePhoneBlacklistEntry"
import {Dialog} from "@sequentech/ui-essentials"
import {theme} from "@sequentech/ui-essentials"
import {Add} from "@mui/icons-material"

const RESOURCE = "sequent_backend_phone_blacklist"

interface EmptyProps {
    onAdd: () => void
}
const Empty: React.FC<EmptyProps> = ({onAdd}) => {
    const {t} = useTranslation()
    return (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" component="p">
                {t("electionEventScreen.ivr.blacklist.emptyMsg")}
            </Typography>
            <Button onClick={onAdd}>
                <Add />
                {t("common.label.add")}
            </Button>
            <Typography variant="body1" component="p">
                {t("common.resources.noResult.askCreate")}
            </Typography>
        </ResourceListStyles.EmptyBox>
    )
}

interface IRowActionsProps {
    canEdit: boolean
    canDelete: boolean
    onEdit: (record: RaRecord) => void
    onDelete: (id: string) => void
    label?: string
}

const RowActions: React.FC<IRowActionsProps> = ({canEdit, canDelete, onEdit, onDelete}) => {
    const record = useRecordContext<RaRecord>()
    if (!record) {
        return null
    }
    return (
        <Box>
            {canEdit && (
                <IconButton
                    onClick={(event) => {
                        event.stopPropagation()
                        onEdit(record)
                    }}
                >
                    <EditIcon />
                </IconButton>
            )}
            {canDelete && (
                <IconButton
                    onClick={(event) => {
                        event.stopPropagation()
                        onDelete(String(record.id))
                    }}
                >
                    <DeleteIcon />
                </IconButton>
            )}
        </Box>
    )
}

export const PhoneBlacklist: React.FC = () => {
    const {t} = useTranslation()
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const notify = useNotify()
    const refresh = useRefresh()

    const [drawerOpen, setDrawerOpen] = useState(false)
    const [phoneE164, setPhoneE164] = useState("")
    const [reason, setReason] = useState("")
    const [phoneError, setPhoneError] = useState("")

    const [editDrawerOpen, setEditDrawerOpen] = useState(false)
    const [editId, setEditId] = useState<string | null>(null)
    const [editPhoneE164, setEditPhoneE164] = useState("")
    const [editReason, setEditReason] = useState("")
    const [openDeleteModal, setOpenDeleteModal] = useState<boolean>(false)
    const [pendingDeleteId, setPendingDeleteId] = useState<string | null>(null)

    const canCreate = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.PHONE_BLACKLIST_CREATE
    )
    const canDelete = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.PHONE_BLACKLIST_DELETE
    )
    const canEdit = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.PHONE_BLACKLIST_UPDATE
    )

    const [insertPhoneBlacklist, {loading: inserting}] = useMutation(CREATE_PHONE_BLACKLIST_ENTRY, {
        context: {headers: {"x-hasura-role": IPermissions.PHONE_BLACKLIST_CREATE}},
        onCompleted: () => {
            notify(t("electionEventScreen.ivr.common.saveSuccess"), {type: "success"})
            setDrawerOpen(false)
            setPhoneE164("")
            setReason("")
            refresh()
        },
        onError: () => {
            notify(t("electionEventScreen.ivr.common.saveError"), {type: "error"})
        },
    })

    const [updatePhoneBlacklist, {loading: updating}] = useMutation(UPDATE_PHONE_BLACKLIST_ENTRY, {
        context: {headers: {"x-hasura-role": IPermissions.PHONE_BLACKLIST_UPDATE}},
        onCompleted: () => {
            notify(t("electionEventScreen.ivr.common.saveSuccess"), {type: "success"})
            setEditDrawerOpen(false)
            setEditId(null)
            refresh()
        },
        onError: () => {
            notify(t("electionEventScreen.ivr.common.saveError"), {type: "error"})
        },
    })

    const [deletePhoneBlacklist] = useMutation(DELETE_PHONE_BLACKLIST_ENTRY, {
        context: {headers: {"x-hasura-role": IPermissions.PHONE_BLACKLIST_DELETE}},
        onCompleted: () => {
            notify(t("electionEventScreen.ivr.common.deleteSuccess"), {type: "success"})
            refresh()
        },
        onError: () => {
            notify(t("electionEventScreen.ivr.common.deleteError"), {type: "error"})
        },
    })

    const handleAdd = () => {
        if (!phoneE164.trim()) {
            setPhoneError(t("electionEventScreen.ivr.blacklist.phoneRequired"))
            return
        }
        setPhoneError("")
        insertPhoneBlacklist({
            variables: {
                election_event_id: record?.id,
                phone_e164: phoneE164.trim(),
                reason: reason.trim() || null,
            },
        })
    }

    const handleOpenEdit = (row: RaRecord) => {
        setEditId(String(row.id))
        setEditPhoneE164(String(row.phone_e164 ?? ""))
        setEditReason(typeof row.reason === "string" ? row.reason : "")
        setEditDrawerOpen(true)
    }

    const handleUpdate = () => {
        if (!editId) {
            return
        }
        updatePhoneBlacklist({
            variables: {
                id: editId,
                reason: editReason.trim() || null,
            },
        })
    }

    const confirmDeleteAction = () => {
        if (pendingDeleteId) {
            deletePhoneBlacklist({variables: {id: pendingDeleteId, election_event_id: record?.id}})
            setPendingDeleteId(null)
        }
        setOpenDeleteModal(false)
    }

    const handleOpenDelete = (id: string) => {
        setPendingDeleteId(id)
        setOpenDeleteModal(true)
    }

    if (!record?.id) {
        return null
    }

    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
            <Alert severity="info">{t("electionEventScreen.ivr.blacklist.infoMsg")}</Alert>
            <List
                resource={RESOURCE}
                filter={{tenant_id: tenantId, election_event_id: record.id}}
                sort={{field: "phone_e164", order: "ASC"}}
                storeKey={false}
                empty={<Empty onAdd={() => setDrawerOpen(true)} />}
                filters={[
                    <TextInput
                        source="phone_e164"
                        label={t("electionEventScreen.ivr.blacklist.columns.phone")}
                    />,
                    <TextInput
                        source="reason"
                        label={t("electionEventScreen.ivr.blacklist.columns.reason")}
                    />,
                    <TextInput
                        source="created_by"
                        label={t("electionEventScreen.ivr.blacklist.columns.createdBy")}
                    />,
                    <DateInput source="created_at@_lte" label="Created Before" />,
                    <DateInput source="created_at@_gte" label="Created After" />,
                ]}
                actions={
                    <ListActions
                        withImport={false}
                        withExport={false}
                        withFilter={true}
                        withAction={canCreate}
                        doAction={() => setDrawerOpen(true)}
                        actionLabel={t("common.label.add")}
                    />
                }
            >
                <DatagridConfigurable
                    empty={
                        <Box
                            sx={{
                                display: "flex",
                                justifyContent: "center",
                                padding: theme.spacing(2),
                            }}
                        >
                            <Typography variant="subtitle1">
                                {t("electionEventScreen.ivr.blacklist.noFilterMatch")}
                            </Typography>
                        </Box>
                    }
                    bulkActionButtons={false}
                >
                    <TextField
                        source="phone_e164"
                        label={t("electionEventScreen.ivr.blacklist.columns.phone")}
                    />
                    <TextField
                        source="reason"
                        label={t("electionEventScreen.ivr.blacklist.columns.reason")}
                    />
                    <TextField
                        source="created_by"
                        label={t("electionEventScreen.ivr.blacklist.columns.createdBy")}
                    />
                    <TextField
                        source="created_at"
                        label={t("electionEventScreen.ivr.blacklist.columns.createdAt")}
                    />
                    {(canEdit || canDelete) && (
                        <RowActions
                            label={t("common.label.actions")}
                            canEdit={canEdit}
                            canDelete={canDelete}
                            onEdit={handleOpenEdit}
                            onDelete={handleOpenDelete}
                        />
                    )}
                </DatagridConfigurable>
            </List>

            <Drawer
                anchor="right"
                open={drawerOpen}
                onClose={() => setDrawerOpen(false)}
                sx={{"& .MuiDrawer-paper": {width: 400}}}
            >
                <Box sx={{p: 3}}>
                    <ElectionHeader title={t("common.label.add")} subtitle="" />
                    <DrawerStyles.Wrapper>
                        <Box sx={{display: "flex", flexDirection: "column", gap: 2, mt: 2}}>
                            <MuiTextField
                                label={t("electionEventScreen.ivr.blacklist.columns.phone")}
                                value={phoneE164}
                                onChange={(e) => {
                                    setPhoneE164(e.target.value)
                                    setPhoneError("")
                                }}
                                error={Boolean(phoneError)}
                                helperText={phoneError}
                                required
                                fullWidth
                            />
                            <MuiTextField
                                label={t("electionEventScreen.ivr.blacklist.columns.reason")}
                                value={reason}
                                onChange={(e) => setReason(e.target.value)}
                                fullWidth
                                multiline
                                rows={3}
                            />
                        </Box>
                        <Box sx={{mt: 3, display: "flex", gap: 2}}>
                            <Button variant="contained" onClick={handleAdd} disabled={inserting}>
                                {t("common.label.save")}
                            </Button>
                            <Button onClick={() => setDrawerOpen(false)}>
                                {t("common.label.cancel")}
                            </Button>
                        </Box>
                    </DrawerStyles.Wrapper>
                </Box>
            </Drawer>

            <Drawer
                anchor="right"
                open={editDrawerOpen}
                onClose={() => setEditDrawerOpen(false)}
                sx={{"& .MuiDrawer-paper": {width: 400}}}
            >
                <Box sx={{p: 3}}>
                    <ElectionHeader title={t("common.label.edit")} subtitle="" />
                    <DrawerStyles.Wrapper>
                        <Box sx={{display: "flex", flexDirection: "column", gap: 2, mt: 2}}>
                            <MuiTextField
                                label={t("electionEventScreen.ivr.blacklist.columns.phone")}
                                value={editPhoneE164}
                                disabled
                                fullWidth
                            />
                            <MuiTextField
                                label={t("electionEventScreen.ivr.blacklist.columns.reason")}
                                value={editReason}
                                onChange={(e) => setEditReason(e.target.value)}
                                fullWidth
                                multiline
                                rows={3}
                            />
                        </Box>
                        <Box sx={{mt: 3, display: "flex", gap: 2}}>
                            <Button variant="contained" onClick={handleUpdate} disabled={updating}>
                                {t("common.label.save")}
                            </Button>
                            <Button onClick={() => setEditDrawerOpen(false)}>
                                {t("common.label.cancel")}
                            </Button>
                        </Box>
                    </DrawerStyles.Wrapper>
                </Box>
            </Drawer>
            <Dialog
                variant="warning"
                open={openDeleteModal}
                ok={String(t("common.label.delete"))}
                cancel={String(t("common.label.cancel"))}
                title={String(t("common.label.warning"))}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmDeleteAction()
                    }
                    setOpenDeleteModal(false)
                }}
            >
                {t("common.message.delete")}
            </Dialog>
        </Box>
    )
}
