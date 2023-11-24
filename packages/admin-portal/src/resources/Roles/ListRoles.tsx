// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    ExportButton,
    SelectColumnsButton,
    TopToolbar,
    Identifier,
} from "react-admin"
import {useTenantStore} from "../../providers/TenantContextProvider"
import {Action, ActionsColumn} from "../../components/ActionButons"
import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import {Drawer} from "@mui/material"
import {EditRole} from "./EditRole"

const OMIT_FIELDS: Array<string> = []

export interface ListRolesProps {
    aside?: ReactElement
    electionEventId?: string
}

export const ListRoles: React.FC<ListRolesProps> = ({aside, electionEventId}) => {
    const [tenantId] = useTenantStore()
    const [open, setOpen] = React.useState(false)
    const [recordId, setRecordId] = React.useState<Identifier | undefined>(undefined)
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
                    <TopToolbar>
                        <SelectColumnsButton />
                        <ExportButton />
                    </TopToolbar>
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
                    <EditRole id={recordId} close={handleCloseEditDrawer} />
                </Drawer>
            </List>
        </>
    )
}
