// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useEffect} from "react"

import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"

import {Drawer} from "@mui/material"
import {useTranslation} from "react-i18next"
import {Dialog} from "@sequentech/ui-essentials"

import {List, TextField, TextInput, useDelete, Identifier, DatagridConfigurable} from "react-admin"

import {ListActions} from "@/components/ListActions"
import {ActionsColumn} from "@/components/ActionButons"
import {SettingselectionsTypesEdit} from "./SettingsElectionsTypesEdit"
import {SettingsElectionsTypesCreate} from "./SettingsElectionsTypesCreate"

const OMIT_FIELDS = ["id", "ballot_eml"]
const Filters: Array<ReactElement> = [<TextInput label="Name" source="name" key={0} />]

export const SettingsElectionsTypes: React.FC<void> = () => {
    const {t} = useTranslation()
    const [deleteOne] = useDelete()

    const [open, setOpen] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [deleteId, setDeleteId] = React.useState<Identifier | undefined>()
    const [closeDrawer, setCloseDrawer] = React.useState("")
    const [recordId, setRecordId] = React.useState<Identifier | undefined>(undefined)

    useEffect(() => {
        if (recordId) {
            setOpen(true)
        }
    }, [recordId])

    const handleCloseCreateDrawer = () => {
        setRecordId(undefined)
        setCloseDrawer(new Date().toISOString())
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
        deleteOne("sequent_backend_area", {id: deleteId})
        setDeleteId(undefined)
    }

    const actions: any[] = [
        {icon: <EditIcon />, action: editAction},
        {icon: <DeleteIcon />, action: deleteAction},
    ]

    return (
        <>
            <List
                filters={Filters}
                actions={
                    <ListActions
                        custom
                        withFilter
                        closeDrawer={closeDrawer}
                        Component={<SettingsElectionsTypesCreate close={handleCloseCreateDrawer} />}
                    />
                }
            >
                <DatagridConfigurable omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="name" />

                    <ActionsColumn actions={actions} />
                </DatagridConfigurable>
            </List>

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
        </>
    )
}
