// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useContext, useEffect} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    TextInput,
    BooleanField,
    Identifier,
    useDelete,
    WrapperField,
    useRefresh,
    useNotify,
} from "react-admin"
import {useTenantStore} from "../../providers/TenantContextProvider"
import {ListActions} from "../../components/ListActions"
import {Drawer} from "@mui/material"
import {Dialog} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"
import {Action, ActionsColumn} from "../../components/ActionButons"
import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import {EditUser} from "./EditUser"
import {CreateUser} from "./CreateUser"
import { AuthContext } from "@/providers/AuthContextProvider"
import { DeleteUserMutation } from "@/gql/graphql"
import { DELETE_USER } from "@/queries/DeleteUser"
import { useMutation } from "@apollo/client"

const OMIT_FIELDS: Array<string> = []

const Filters: Array<ReactElement> = [
    <TextInput label="Name" source="name" key={0} />,
    <TextInput label="Description" source="description" key={1} />,
    <TextInput label="ID" source="id" key={2} />,
    <TextInput label="Type" source="type" key={3} />,
    <TextInput source="election_event_id" key={3} />,
]

export interface ListUsersProps {
    aside?: ReactElement
    electionEventId?: string
}

export const ListUsers: React.FC<ListUsersProps> = ({aside, electionEventId}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const [deleteOne] = useDelete()

    const [open, setOpen] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [deleteId, setDeleteId] = React.useState<Identifier | undefined>()
    const [openDrawer, setOpenDrawer] = React.useState(false)
    const [recordId, setRecordId] = React.useState<Identifier | undefined>(undefined)
    const authContext = useContext(AuthContext)
    const refresh = useRefresh()
    const [deleteUser] = useMutation<DeleteUserMutation>(DELETE_USER)
    const notify = useNotify()

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

    useEffect(() => {
        if (recordId) {
            setTimeout(() => {
                setOpen(true)
            }, 400)
        }
    }, [recordId])

    const editAction = (id: Identifier) => {
        setRecordId(id)
    }

    const deleteAction = (id: Identifier) => {
        if (!electionEventId && authContext.userId === id) {
            return
        }
        // deleteOne("sequent_backend_area", {id})
        setOpenDeleteModal(true)
        setDeleteId(id)
    }

    const confirmDeleteAction = async () => {
        const {errors} =  await deleteUser({
            variables: {
                tenantId: tenantId,
                electionEventId: electionEventId,
                userId: deleteId,
            },
        })
        if (errors) {
            notify(t(`usersAndRolesScreen.${electionEventId? 'voters' : 'users'}.notifications.deleteError`), {type: "error"})
            console.log(`Error creating user: ${errors}`)
            return
        }
        notify(t(`usersAndRolesScreen.${electionEventId? 'voters' : 'users'}.notifications.deleteSuccess`), {type: "success"})
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
                resource="user"
                empty={false}
                actions={
                    <ListActions
                        withImport={false}
                        open={openDrawer}
                        setOpen={setOpenDrawer}
                        Component={
                            <CreateUser
                                electionEventId={electionEventId}
                                close={handleCloseCreateDrawer}
                            />
                        }
                    />
                }
                // actions={
                //     <TopToolbar>
                //         <SelectColumnsButton />
                //         <ExportButton />
                //     </TopToolbar>
                // }
                filter={{tenant_id: tenantId, election_event_id: electionEventId}}
                aside={aside}
                filters={Filters}
            >
                <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={<></>}>
                    <TextField source="id" />
                    <TextField source="email" />
                    <BooleanField source="email_verified" />
                    <BooleanField source="enabled" />
                    <TextField source="first_name" />
                    <TextField source="last_name" />
                    <TextField source="username" />

                    <WrapperField source="actions" label="Actions">
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
                <EditUser
                    id={recordId}
                    electionEventId={electionEventId}
                    close={handleCloseEditDrawer}
                />
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
                {t(`usersAndRolesScreen.${electionEventId? 'voters' : 'users'}.delete.body`)}
            </Dialog>
        </>
    )
}
