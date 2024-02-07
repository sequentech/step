import {Action, ActionsColumn} from "../../components/ActionButons"
import {Button, Drawer, Typography} from "@mui/material"
import {
    DatagridConfigurable,
    FunctionField,
    Identifier,
    List,
    RaRecord,
    TextField,
    TextInput,
    WrapperField,
    useDelete,
    useNotify,
    useRecordContext,
    useRefresh,
} from "react-admin"
import {Dialog, IconButton} from "@sequentech/ui-essentials"
// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useContext, useEffect} from "react"

import {AreaContestItems} from "@/components/AreaContestItems"
import {AuthContext} from "@/providers/AuthContextProvider"
import {CreateArea} from "./CreateArea"
import DeleteIcon from "@mui/icons-material/Delete"
import {EditArea} from "./EditArea"
import EditIcon from "@mui/icons-material/Edit"
import {IPermissions} from "@/types/keycloak"
import {ListActions} from "../../components/ListActions"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {Sequent_Backend_Election_Event} from "../../gql/graphql"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {useParams} from "react-router"
import {useTenantStore} from "../../providers/TenantContextProvider"
import {useTranslation} from "react-i18next"

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
    const refresh = useRefresh()

    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const notify = useNotify()

    const [tenantId] = useTenantStore()
    const [deleteOne] = useDelete()

    const [open, setOpen] = React.useState(false)
    const [openCreate, setOpenCreate] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [deleteId, setDeleteId] = React.useState<Identifier | undefined>()
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)
    const [recordId, setRecordId] = React.useState<Identifier | undefined>(undefined)

    const authContext = useContext(AuthContext)
    const canView = authContext.isAuthorized(true, tenantId, IPermissions.AREA_READ)
    const canCreate = authContext.isAuthorized(true, tenantId, IPermissions.AREA_WRITE)

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

    const createAction = () => {
        setOpenCreate(true)
    }

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t("areas.empty.header")}
            </Typography>
            {canCreate && (
                <>
                    <Button onClick={createAction} className="area-add-button">
                        <IconButton icon={faPlus} fontSize="24px" />
                        {t("areas.empty.action")}
                    </Button>
                    <Typography variant="body1" paragraph>
                        {t("common.resources.noResult.askCreate")}
                    </Typography>
                </>
            )}
        </ResourceListStyles.EmptyBox>
    )

    if (!canView) {
        return <Empty />
    }

    const handleCloseCreateDrawer = () => {
        setRecordId(undefined)
        setOpenCreate(false)
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
    }

    const deleteAction = (id: Identifier) => {
        // deleteOne("sequent_backend_area", {id})
        setOpenDeleteModal(true)
        setDeleteId(id)
    }

    const confirmDeleteAction = () => {
        deleteOne(
            "sequent_backend_area",
            {id: deleteId},
            {
                onSuccess() {
                    refresh()
                },
                onError() {
                    notify(t("areas.common.deleteError"), {type: "error"})
                    refresh()
                },
            }
        )
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
                        open={openDrawer}
                        setOpen={setOpenDrawer}
                        Component={<CreateArea record={record} close={handleCloseCreateDrawer} />}
                    />
                }
                empty={<Empty />}
                sx={{flexGrow: 2}}
                filter={{
                    tenant_id: tenantId || undefined,
                    election_event_id: record?.id || undefined,
                }}
                filters={Filters}
            >
                <DatagridConfigurable omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="name" className="area-name" />
                    <TextField source="description" />

                    <FunctionField
                        label={t("areas.sequent_backend_area_contest")}
                        render={(record: any) => <AreaContestItems record={record} />}
                    />

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
                <EditArea id={recordId} electionEventId={id} close={handleCloseEditDrawer} />
            </Drawer>
            <Drawer
                anchor="right"
                open={openCreate}
                onClose={handleCloseCreateDrawer}
                PaperProps={{
                    sx: {width: "40%"},
                }}
            >
                <CreateArea record={record} close={handleCloseCreateDrawer} />
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
