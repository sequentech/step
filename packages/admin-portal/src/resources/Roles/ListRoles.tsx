// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    Identifier,
    useGetList,
} from "react-admin"
import {useTenantStore} from "../../providers/TenantContextProvider"
import {Action, ActionsColumn} from "../../components/ActionButons"
import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import {Drawer} from "@mui/material"
import {EditRole} from "./EditRole"
import {IPermission} from "sequent-core"
import { ListActions } from "@/components/ListActions"
import { CreateRole } from "./CreateRole"

const OMIT_FIELDS: Array<string> = []

export interface ListRolesProps {
    aside?: ReactElement
    electionEventId?: string
}

export const ListRoles: React.FC<ListRolesProps> = ({aside, electionEventId}) => {
    const [tenantId] = useTenantStore()
    const [open, setOpen] = React.useState(false)
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)
    const [recordId, setRecordId] = React.useState<Identifier | undefined>(undefined)
    const {
        data: permissions,
        total,
        isLoading,
        error,
        refetch,
    } = useGetList<IPermission & {id: string}>("permission", {
        filter: {tenant_id: tenantId},
    })

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

    const deleteAction = (id: Identifier) => {}

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
                        Component={
                            <CreateRole close={handleCloseCreateDrawer}
                            />
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
        </>
    )
}
