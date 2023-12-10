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
    WrapperField,
    useRefresh,
    useNotify,
    useGetList,
    FunctionField,
} from "react-admin"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ListActions} from "@/components/ListActions"
import {Button, Chip, Typography} from "@mui/material"
import {Dialog} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"
import {Action, ActionsColumn} from "@/components/ActionButons"
import EditIcon from "@mui/icons-material/Edit"
import MailIcon from "@mui/icons-material/Mail"
import DeleteIcon from "@mui/icons-material/Delete"
import {EditUser} from "./EditUser"
import {SendCommunication} from "./SendCommunication"
import {CreateUser} from "./CreateUser"
import {AuthContext} from "@/providers/AuthContextProvider"
import {DeleteUserMutation} from "@/gql/graphql"
import {DELETE_USER} from "@/queries/DeleteUser"
import {useMutation} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {IRole, IUser} from "sequent-core"

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

    const [open, setOpen] = React.useState(false)
    const [openNew, setOpenNew] = React.useState(false)
    const [openSendCommunication, setOpenSendCommunication] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [deleteId, setDeleteId] = React.useState<string | undefined>()
    const [openDrawer, setOpenDrawer] = React.useState(false)
    const [recordId, setRecordId] = React.useState<string | undefined>(undefined)
    const authContext = useContext(AuthContext)
    const refresh = useRefresh()
    const [deleteUser] = useMutation<DeleteUserMutation>(DELETE_USER)
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
                    <Button onClick={() => setOpenNew(true)}>
                        <ResourceListStyles.CreateIcon icon={faPlus} />
                        {t(
                            `usersAndRolesScreen.${
                                electionEventId ? "voters" : "users"
                            }.create.subtitle`
                        )}
                    </Button>
                </>
            ) : null}
        </ResourceListStyles.EmptyBox>
    )

    const handleClose = () => {
        setRecordId(undefined)
        setOpenSendCommunication(false)
        setOpenDrawer(false)
        setOpenNew(false)
        setOpen(false)
    }

    const editAction = (id: Identifier) => {
        setOpen(true)
        setOpenNew(false)
        setOpenDeleteModal(false)
        setOpenSendCommunication(false)
        setRecordId(id as string)
    }

    const sendCommunicationAction = (id: Identifier) => {
        setOpen(false)
        setOpenNew(false)
        setOpenDeleteModal(false)
        setOpenSendCommunication(true)
        setRecordId(id as string)
    }

    const deleteAction = (id: Identifier) => {
        if (!electionEventId && authContext.userId === id) {
            return
        }
        setOpen(false)
        setOpenNew(false)
        setOpenSendCommunication(false)
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
            action: sendCommunicationAction,
            showAction: (id: Identifier) => canSendCommunications,
        },
        {
            icon: <EditIcon />,
            action: editAction,
            showAction: (id: Identifier) => canEditUsers,
        },
        {
            icon: <DeleteIcon />,
            action: deleteAction,
            showAction: (id: Identifier) => canEditUsers,
        },
    ]

    return (
        <>
            <List
                resource="user"
                empty={<Empty />}
                actions={
                    <ListActions
                        withImport={false}
                        open={openDrawer}
                        setOpen={setOpenDrawer}
                        Component={
                            <CreateUser electionEventId={electionEventId} close={handleClose} />
                        }
                    />
                }
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
                    {electionEventId && (
                        <FunctionField
                            label={t("usersAndRolesScreen.users.fields.area")}
                            render={(record: IUser) => <Chip label={record?.area?.name || ""} />}
                        />
                    )}

                    <WrapperField source="actions" label="Actions">
                        <ActionsColumn actions={actions} />
                    </WrapperField>
                </DatagridConfigurable>
            </List>

            <ResourceListStyles.Drawer anchor="right" open={open} onClose={handleClose}>
                <EditUser
                    id={recordId}
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
                    id={recordId}
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
        </>
    )
}
