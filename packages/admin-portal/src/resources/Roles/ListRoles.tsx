// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useContext} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    Identifier,
    useGetList,
    useNotify,
    useRefresh,
    TextInput,
} from "react-admin"
import {useTenantStore} from "../../providers/TenantContextProvider"
import {Action, ActionsColumn} from "../../components/ActionButons"
import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import {Drawer} from "@mui/material"
import {EditRole} from "./EditRole"
import {IPermission} from "@sequentech/ui-core"
import {ListActions} from "@/components/ListActions"
import {CreateRole} from "./CreateRole"
import {Dialog} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"
import {useMutation} from "@apollo/client"
import {DELETE_ROLE} from "@/queries/DeleteRole"
import {DeleteRoleMutation} from "@/gql/graphql"
import {IPermissions} from "@/types/keycloak"
import {AuthContext} from "@/providers/AuthContextProvider"

const OMIT_FIELDS: Array<string> = []

export interface ListRolesProps {
    aside?: ReactElement
}

export const ListRoles: React.FC<ListRolesProps> = ({aside}) => {
    const [tenantId] = useTenantStore()
    const [open, setOpen] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)
    const [recordId, setRecordId] = React.useState<Identifier | undefined>(undefined)
    const {t} = useTranslation()
    const [deleteId, setDeleteId] = React.useState<Identifier | undefined>()
    const {
        data: permissions,
        total,
        isLoading,
        error,
        refetch,
    } = useGetList<IPermission & {id: string}>("permission", {
        filter: {tenant_id: tenantId},
    })
    const authContext = useContext(AuthContext)
    const canCreateRole = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ROLE_CREATE
    )

    const [deleteRole] = useMutation<DeleteRoleMutation>(DELETE_ROLE)
    const notify = useNotify()
    const refresh = useRefresh()

    const handleCloseCreateDrawer = () => {
        setRecordId(undefined)
        setOpenDrawer(false)
    }

    const handleCloseEditDrawer = () => {
        setOpen(false)
        setTimeout(() => {
            setRecordId(undefined)
        }, 400)
    }

    const editAction = (id: Identifier) => {
        setRecordId(id)
        setOpen(true)
    }

    const deleteAction = (id: Identifier) => {
        setOpenDeleteModal(true)
        setDeleteId(id)
    }

    const confirmDeleteAction = async () => {
        const {errors} = await deleteRole({
            variables: {
                tenantId: tenantId,
                roleId: deleteId,
            },
        })
        if (errors) {
            notify(t(`usersAndRolesScreen.roles.notifications.deleteError`), {type: "error"})
            console.log(`Error deleting role: ${errors}`)
            return
        }
        notify(t(`usersAndRolesScreen.roles.notifications.deleteSuccess`), {type: "success"})
        setDeleteId(undefined)
        refresh()
    }

    const actions: Action[] = [
        {icon: <EditIcon />, action: editAction},
        {icon: <DeleteIcon />, action: deleteAction},
    ]

    return (
        <>
            <List
                resource="role"
                actions={
                    <ListActions
                        withImport={false}
                        withFilter={false}
                        open={openDrawer}
                        setOpen={setOpenDrawer}
                        withComponent={canCreateRole}
                        Component={
                            <CreateRole close={handleCloseCreateDrawer} permissions={permissions} />
                        }
                    />
                }
                filter={{tenant_id: tenantId}}
                aside={aside}
            >
                <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={<></>}>
                    <TextField source="name" />
                    <TextField source="id" />
                    <ActionsColumn actions={actions} />
                </DatagridConfigurable>
                <Drawer
                    anchor="right"
                    open={open}
                    onClose={handleCloseEditDrawer}
                    PaperProps={{
                        sx: {width: "40%"},
                    }}
                >
                    <EditRole
                        id={recordId}
                        close={handleCloseEditDrawer}
                        permissions={permissions}
                    />
                </Drawer>
            </List>
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
                {t(`usersAndRolesScreen.roles.delete.body`)}
            </Dialog>
        </>
    )
}
