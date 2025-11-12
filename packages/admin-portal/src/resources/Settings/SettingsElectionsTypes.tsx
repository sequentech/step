// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useContext, useEffect} from "react"

import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import AddIcon from "@mui/icons-material/Add"

import {Box, Button, Drawer, Icon, Typography} from "@mui/material"
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

import {SettingselectionsTypesEdit} from "./SettingsElectionsTypesEdit"
import {SettingsElectionsTypesCreate} from "./SettingsElectionsTypesCreate"

export const EmptyBox = styled(Box)`
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

    const canWriteTenant = authContext.isAuthorized(true, tenantId, IPermissions.TENANT_WRITE)

    return {
        canWriteTenant,
    }
}

const OMIT_FIELDS = ["id", "ballot_eml"]
const Filters: Array<ReactElement> = [<TextInput label="Name" source="name" key={0} />]

export const SettingsElectionsTypes: React.FC<void> = () => {
    const {t} = useTranslation()
    const [deleteOne] = useDelete()
    const {canWriteTenant} = useActionPermissions()

    const [open, setOpen] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [deleteId, setDeleteId] = React.useState<Identifier | undefined>()
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)
    const [recordId, setRecordId] = React.useState<Identifier | undefined>(undefined)

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
        deleteOne("sequent_backend_election_type", {id: deleteId})
        setDeleteId(undefined)
    }

    const actions: any[] = [
        {icon: <EditIcon />, action: editAction},
        {icon: <DeleteIcon />, action: deleteAction},
    ]

    const CreateButton = () => (
        <Button onClick={handleOpenCreateDrawer} startIcon={<AddIcon sx={{fontSize: "24px"}} />}>
            {t("electionTypeScreen.common.createNew")}
        </Button>
    )

    const Empty = () => (
        <EmptyBox m={1}>
            <Typography variant="h4" paragraph>
                {t("electionTypeScreen.common.emptyHeader")}
            </Typography>
            {canWriteTenant ? (
                <>
                    <Typography variant="body1" paragraph>
                        {t("electionTypeScreen.common.emptyBody")}
                    </Typography>
                    <CreateButton />
                </>
            ) : null}
        </EmptyBox>
    )

    if (!canWriteTenant) {
        return <Empty />
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
                        Component={<SettingsElectionsTypesCreate close={handleCloseCreateDrawer} />}
                    />
                }
                empty={<Empty />}
            >
                <DatagridConfigurable omit={OMIT_FIELDS}>
                    <TextField source="id" />
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
                <SettingsElectionsTypesCreate close={handleCloseCreateDrawer} />
            </Drawer>

            <Drawer
                anchor="right"
                open={open}
                onClose={handleCloseEditDrawer}
                PaperProps={{
                    sx: {width: "40%"},
                }}
            >
                <SettingselectionsTypesEdit id={recordId} close={handleCloseEditDrawer} />
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
        </>
    )
}
