// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
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
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {Drawer} from "@mui/material"
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
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)
    const [recordId, setRecordId] = React.useState<string | undefined>(undefined)

    useEffect(() => {
        if (recordId) {
            setOpen(true)
        }
    }, [recordId])

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
        setRecordId(id as string)
    }

    const deleteAction = (id: Identifier) => {
        // deleteOne("sequent_backend_area", {id})
        setOpenDeleteModal(true)
        setDeleteId(id as string)
    }

    const confirmDeleteAction = () => {
        deleteOne(
            "sequent_backend_area",
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
        {icon: <EditIcon />, action: editAction},
        {icon: <DeleteIcon />, action: deleteAction},
    ]

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
                        open={openDrawer}
                        setOpen={setOpenDrawer}
                        Component={
                            <CreateSupportMaterial
                                record={record}
                                close={handleCloseCreateDrawer}
                            />
                        }
                    />
                }
                empty={false}
                sx={{flexGrow: 2}}
                filter={{
                    tenant_id: tenantId || undefined,
                    election_event_id: record?.id || undefined,
                }}
                filters={Filters}
            >
                <ResetFilters />
                <DatagridConfigurable omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <BooleanField
                        source={"is_hidden"}
                        label={t("materials.fields.isHidden")}
                        sortable={false}
                    />
                    <TextField
                        source={`data.title_i18n[${i18n.language}]`}
                        label={t("common.label.title")}
                        sortable={false}
                    />
                    <TextField
                        source={`data.subtitle_i18n[${i18n.language}]`}
                        label={t("common.label.subtitle")}
                        sortable={false}
                    />
                    {/* <TextField source={"kind"} label={t("common.label.kind")} sortable={false} /> */}

                    <FunctionField
                        label={t("common.label.kind")}
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
