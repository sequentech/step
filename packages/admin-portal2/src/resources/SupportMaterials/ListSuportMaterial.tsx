// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useEffect} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    TextInput,
    Identifier,
    RaRecord,
    useRecordContext,
    useDelete,
    WrapperField,
    FunctionField,
    useRefresh,
    BooleanField,
    // Button,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {Drawer, IconButton, Typography, Button} from "@mui/material"
import {EditSupportMaterial} from "./EditSuportMaterial"
import {CreateSupportMaterial} from "./CreateSupportMaterial"
import {Sequent_Backend_Election_Event} from "../../gql/graphql"
import {Dialog} from "@sequentech/ui-essentials"
import {Action, ActionsColumn} from "../../components/ActionButons"
import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "../../providers/TenantContextProvider"
import {useParams} from "react-router"
import VideoFileIcon from "@mui/icons-material/VideoFile"
import AudioFileIcon from "@mui/icons-material/AudioFile"
import PictureAsPdfIcon from "@mui/icons-material/PictureAsPdf"
import ImageIcon from "@mui/icons-material/Image"
import {ResetFilters} from "@/components/ResetFilters"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {IconButton as IconButtonSequent} from "@sequentech/ui-essentials"
import {useSuportMaterialPermissions} from "./useSuporMaterialPermissions"

const OMIT_FIELDS = ["id", "ballot_eml"]

const Filters: Array<ReactElement> = [
    <TextInput label="Name" source="name" key={0} />,
    <TextInput label="Description" source="description" key={1} />,
    <TextInput label="ID" source="id" key={2} />,
    <TextInput label="Type" source="type" key={3} />,
    <TextInput source="election_event_id" key={3} />,
]

export interface ListAreaProps {
    electionEventId?: Identifier | undefined
}

export const ListSupportMaterials: React.FC<ListAreaProps> = (props) => {
    const {t, i18n} = useTranslation()
    const {id} = useParams()
    const refresh = useRefresh()

    // const record = useRecordContext<Sequent_Backend_Support_Material>()
    const record = useRecordContext<Sequent_Backend_Election_Event>()

    const [tenantId] = useTenantStore()
    const [deleteOne, {isLoading, error}] = useDelete()

    const [open, setOpen] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [deleteId, setDeleteId] = React.useState<string | undefined>()
    const [openCreate, setOpenCreate] = React.useState<boolean>(false)
    const [recordId, setRecordId] = React.useState<string | undefined>(undefined)

    const {
        canReadSuportMaterial,
        canWriteSuportMaterial,
        canCreateSuportMaterial,
        canDeleteSuportMaterial,
    } = useSuportMaterialPermissions()

    useEffect(() => {
        if (recordId) {
            setOpen(true)
        }
    }, [recordId])

    const handleCloseCreateDrawer = () => {
        setOpenCreate(false)
    }

    const handleCreateDrawer = () => {
        setOpenCreate(true)
    }

    const handleCloseEditDrawer = () => {
        setOpen(false)
        setTimeout(() => {
            setRecordId(undefined)
        }, 400)
    }

    const editAction = (id: Identifier) => {
        setRecordId(id as string)
    }

    const deleteAction = (id: Identifier) => {
        // deleteOne("sequent_backend_area", {id})
        setOpenDeleteModal(true)
        setDeleteId(id as string)
    }

    const confirmDeleteAction = () => {
        deleteOne(
            "sequent_backend_support_material",
            {id: deleteId},
            {
                onSuccess() {
                    refresh()
                },
            }
        )
        setDeleteId(undefined)
    }

    const actions: Action[] = [
        {icon: <EditIcon />, action: editAction, showAction: () => canWriteSuportMaterial},
        {icon: <DeleteIcon />, action: deleteAction, showAction: () => canDeleteSuportMaterial},
    ]

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t("materials.empty.header")}
            </Typography>
            {/* {canPublishCreate && canReadPublish && ( */}
            <>
                <Button onClick={handleCreateDrawer} className="publish-add-button">
                    <IconButtonSequent icon={faPlus as any} fontSize="24px" />
                    {t("materials.empty.action")}
                </Button>
                <Typography variant="body1" paragraph>
                    {t("common.resources.noResult.askCreate")}
                </Typography>
            </>
            {/* )} */}
        </ResourceListStyles.EmptyBox>
    )

    return (
        <>
            <List
                resource="sequent_backend_support_material"
                actions={
                    <ListActions
                        withImport={false}
                        withExport={false}
                        withFilter={false}
                        withColumns={false}
                        open={openCreate}
                        setOpen={setOpenCreate}
                        Component={
                            <CreateSupportMaterial
                                record={record}
                                close={handleCloseCreateDrawer}
                            />
                        }
                        withComponent={canCreateSuportMaterial}
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
                {/* <ResetFilters /> */}
                <DatagridConfigurable omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <BooleanField
                        source={"is_hidden"}
                        label={String(t("materials.fields.isHidden"))}
                        sortable={false}
                    />
                    <TextField
                        source={`data.title_i18n[${i18n.language}]`}
                        label={String(t("common.label.title"))}
                        sortable={false}
                    />
                    <TextField
                        source={`data.subtitle_i18n[${i18n.language}]`}
                        label={String(t("common.label.subtitle"))}
                        sortable={false}
                    />
                    {/* <TextField source={"kind"} label={String(t("common.label.kind"))} sortable={false} /> */}

                    <FunctionField
                        label={String(t("common.label.kind"))}
                        render={(record: any) => {
                            return record.kind.includes("image") ? (
                                <ImageIcon sx={{fontSize: "36px"}} />
                            ) : record.kind.includes("pdf") ? (
                                <PictureAsPdfIcon sx={{fontSize: "36px"}} />
                            ) : record.kind.includes("video") ? (
                                <VideoFileIcon sx={{fontSize: "36px"}} />
                            ) : record.kind.includes("audio") ? (
                                <AudioFileIcon sx={{fontSize: "36px"}} />
                            ) : null
                        }}
                    />

                    <WrapperField source="actions" label="Actions" sortable={false}>
                        <ActionsColumn actions={actions} />
                    </WrapperField>
                </DatagridConfigurable>
            </List>
            <Drawer
                anchor="right"
                open={openCreate}
                onClose={handleCloseCreateDrawer}
                PaperProps={{
                    sx: {width: "40%"},
                }}
            >
                <CreateSupportMaterial record={record} close={handleCloseCreateDrawer} />
            </Drawer>
            <Drawer
                anchor="right"
                open={open}
                onClose={handleCloseEditDrawer}
                PaperProps={{
                    sx: {width: "40%"},
                }}
            >
                <EditSupportMaterial
                    id={recordId}
                    electionEventId={id}
                    close={handleCloseEditDrawer}
                />
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
