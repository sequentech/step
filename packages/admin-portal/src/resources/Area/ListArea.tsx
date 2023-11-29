// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useEffect} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    ReferenceField,
    ReferenceManyField,
    TextInput,
    Identifier,
    RaRecord,
    useRecordContext,
    useDelete,
    WrapperField,
    Datagrid,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {Drawer} from "@mui/material"
import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import {ChipList} from "../../components/ChipList"
import {EditArea} from "./EditArea"
import {CreateArea} from "./CreateArea"
import {Sequent_Backend_Election_Event} from "../../gql/graphql"
import {Dialog} from "@sequentech/ui-essentials"
import {Action, ActionsColumn} from "../../components/ActionButons"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "../../providers/TenantContextProvider"
import { useParams } from 'react-router'

const OMIT_FIELDS = ["id", "ballot_eml"]

const Filters: Array<ReactElement> = [
    <TextInput label="Name" source="name" key={0} />,
    <TextInput label="Description" source="description" key={1} />,
    <TextInput label="ID" source="id" key={2} />,
    <TextInput label="Type" source="type" key={3} />,
    <TextInput source="election_event_id" key={3} />,
]

export interface ListAreaProps {
    aside?: ReactElement
}

export const ListArea: React.FC<ListAreaProps> = (props) => {
    const {t} = useTranslation()
    const {id} = useParams()
    
    const record = useRecordContext<Sequent_Backend_Election_Event>()

    const [tenantId] = useTenantStore()
    const [deleteOne, {isLoading, error}] = useDelete()

    const [open, setOpen] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [deleteId, setDeleteId] = React.useState<Identifier | undefined>()
    const [closeDrawer, setCloseDrawer] = React.useState("")
    const [recordId, setRecordId] = React.useState<Identifier | undefined>(undefined)

    // const rowClickHandler = generateRowClickHandler(["election_event_id"])
    const rowClickHandler = (id: Identifier, resource: string, record: RaRecord) => {
        setRecordId(id)
        return ""
    }

    useEffect(() => {
        if (recordId) {
            setOpen(true)
        }
    }, [recordId])

    const handleCloseCreateDrawer = () => {
        setRecordId(undefined)
        setCloseDrawer(new Date().toISOString())
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
        // deleteOne("sequent_backend_area", {id})
        setOpenDeleteModal(true)
        setDeleteId(id)
    }

    const confirmDeleteAction = () => {
        deleteOne("sequent_backend_area", {id: deleteId})
        setDeleteId(undefined)
    }

    const actions: Action[] = [
        {icon: <EditIcon />, action: editAction},
        {icon: <DeleteIcon />, action: deleteAction},
    ]

    return (
        <>
            <List
                resource="sequent_backend_area"
                actions={
                    <ListActions
                        withImport={false}
                        closeDrawer={closeDrawer}
                        Component={<CreateArea record={record} close={handleCloseCreateDrawer} />}
                    />
                }
                sx={{flexGrow: 2}}
                filter={{
                    tenant_id: tenantId || undefined,
                    election_event_id: id || undefined,
                }}
                filters={Filters}
            >
                <DatagridConfigurable omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="name" />
                    <TextField source="description" />
                    <TextField source="type" />
                    
                    <ReferenceManyField
                        label="Area Contests"
                        reference="sequent_backend_area_contest"
                        target="area_id"
                    >
                        <ChipList
                            source="sequent_backend_area_contest"
                            filterFields={["election_event_id", "area_id"]}
                        />
                    </ReferenceManyField>

                    <WrapperField source="actions" label="actions">
                        <ActionsColumn actions={actions} />
                    </WrapperField>
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
                <EditArea id={recordId} electionEventId={id} close={handleCloseEditDrawer} />
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
