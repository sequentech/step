// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useContext, useEffect} from "react"
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
    useNotify,
    useListContext,
    useGetList,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {Box, Button, Chip, Drawer, Typography} from "@mui/material"
import {EditArea} from "./EditArea"
import {CreateArea} from "./CreateArea"
import {
    ImportAreasMutation,
    Sequent_Backend_Area,
    Sequent_Backend_Area_Contest,
    Sequent_Backend_Election_Event,
} from "../../gql/graphql"
import {Dialog, IconButton} from "@sequentech/ui-essentials"
import {Action, ActionsColumn} from "../../components/ActionButons"
import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "../../providers/TenantContextProvider"
import {useParams} from "react-router"
import {AreaContestItems} from "@/components/AreaContestItems"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {IPermissions} from "@/types/keycloak"
import {AuthContext} from "@/providers/AuthContextProvider"
import {ImportDataDrawer} from "@/components/election-event/import-data/ImportDataDrawer"
import {useMutation} from "@apollo/client"
import {IMPORT_AREAS} from "@/queries/ImportAreas"
import styled from "@emotion/styled"
import {UPSERT_AREAS} from "@/queries/UpsertAreas"
import {isEqual} from "lodash"

const ActionsBox = styled(Box)`
    display: flex;
    flex-direction: row;
    gap: 8px;
`

const OMIT_FIELDS = ["id", "ballot_eml"]

const ParentAreaItem: React.FC<{
    record: Sequent_Backend_Area
    parentAreas?: Array<Sequent_Backend_Area>
}> = ({record, parentAreas}) => {
    let parentArea =
        (record?.parent_id && parentAreas?.find((area) => area.id === record?.parent_id)) ?? null

    return <>{parentArea ? <Chip label={parentArea?.name} /> : null}</>
}

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

const ParentIdsSetter: React.FC<{
    parentAreaIds: Array<string>
    setParentAreaIds: (val: Array<string>) => void
}> = ({parentAreaIds, setParentAreaIds}) => {
    const {data: records, isLoading} = useListContext<Sequent_Backend_Area>()

    React.useEffect(() => {
        if (!isLoading) {
            let newIds = records.map((record) => record.parent_id as string).filter((id) => id)
            if (!isEqual(newIds, parentAreaIds)) {
                setParentAreaIds(newIds)
            }
        }
    }, [records, isLoading])

    return <></>
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
    const [parentAreaIds, setParentAreaIds] = React.useState<Array<string>>([])
    const [openCreate, setOpenCreate] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [deleteId, setDeleteId] = React.useState<Identifier | undefined>()
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)
    const [recordId, setRecordId] = React.useState<Identifier | undefined>(undefined)
    const [openImportDrawer, setOpenImportDrawer] = React.useState(false)
    const [openUpsertDrawer, setOpenUpsertDrawer] = React.useState(false)
    const [importAreas] = useMutation<ImportAreasMutation>(IMPORT_AREAS, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.AREA_WRITE,
            },
        },
    })
    const [upsertAreas] = useMutation<ImportAreasMutation>(UPSERT_AREAS, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.AREA_WRITE,
            },
        },
    })

    const {data: parentAreas, refetch} = useGetList<Sequent_Backend_Area>(
        "sequent_backend_area",
        {
            sort: {field: "id", order: "DESC"},
            filter: {
                tenant_id: tenantId,
                election_event_id: record?.id,
                id: {
                    format: "hasura-raw-query",
                    value: {_in: parentAreaIds},
                },
            },
            pagination: {
                page: 1,
                perPage: 1e3,
            },
        },
        {
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const authContext = useContext(AuthContext)
    const canView = authContext.isAuthorized(true, tenantId, IPermissions.AREA_READ)
    const canCreate = authContext.isAuthorized(true, tenantId, IPermissions.AREA_WRITE)

    useEffect(() => {
        if (recordId) {
            setOpen(true)
        }
    }, [recordId])

    useEffect(() => {
        refetch()
    }, [refetch, parentAreaIds])

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
                    <ActionsBox>
                        <Button onClick={createAction}>
                            <IconButton icon={faPlus} fontSize="24px" />
                            {t("areas.empty.action")}
                        </Button>
                        <Button onClick={() => setOpenImportDrawer(true)}>
                            <IconButton icon={faPlus} fontSize="24px" />
                            {t("common.label.import")}
                        </Button>
                    </ActionsBox>
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

    const handleImportAreas = async (documentId: string, sha256: string): Promise<void> => {
        let {errors} = await importAreas({
            variables: {
                documentId,
                electionEventId: record.id,
            },
        })

        refresh()

        if (!errors) {
            notify(t("electionEventScreen.importAreas.importSuccess"), {type: "success"})
        } else {
            notify(t("electionEventScreen.importAreas.importError"), {type: "error"})
        }
    }

    const handleUpsertAreas = async (documentId: string, sha256: string): Promise<void> => {
        let {errors} = await upsertAreas({
            variables: {
                documentId,
                electionEventId: record.id,
            },
        })

        refresh()

        if (!errors) {
            notify(t("electionEventScreen.importAreas.importSuccess"), {type: "success"})
        } else {
            notify(t("electionEventScreen.importAreas.importError"), {type: "error"})
        }
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
                        withImport
                        doImport={() => setOpenImportDrawer(true)}
                        open={openDrawer}
                        setOpen={setOpenDrawer}
                        Component={<CreateArea record={record} close={handleCloseCreateDrawer} />}
                        extraActions={[
                            <Button onClick={() => setOpenUpsertDrawer(true)} key="upsert">
                                {t("electionEventScreen.importAreas.upsert")}
                            </Button>,
                        ]}
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
                <ParentIdsSetter
                    parentAreaIds={parentAreaIds}
                    setParentAreaIds={setParentAreaIds}
                />
                <DatagridConfigurable omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="name" />
                    <TextField source="description" />

                    <FunctionField
                        label={t("areas.sequent_backend_area_contest")}
                        render={(record: Sequent_Backend_Area) => (
                            <AreaContestItems record={record} />
                        )}
                    />
                    <FunctionField
                        label={t("areas.parent_areas")}
                        render={(record: Sequent_Backend_Area) => (
                            <ParentAreaItem record={record} parentAreas={parentAreas} />
                        )}
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
            <ImportDataDrawer
                open={openImportDrawer}
                closeDrawer={() => setOpenImportDrawer(false)}
                title="electionEventScreen.importAreas.title"
                subtitle="electionEventScreen.importAreas.subtitle"
                paragraph="electionEventScreen.importAreas.areaParagraph"
                doImport={handleImportAreas}
                errors={null}
            />
            <ImportDataDrawer
                open={openUpsertDrawer}
                closeDrawer={() => setOpenUpsertDrawer(false)}
                title="electionEventScreen.importAreas.title"
                subtitle="electionEventScreen.importAreas.subtitle"
                paragraph="electionEventScreen.importAreas.areaParagraph"
                doImport={handleUpsertAreas}
                errors={null}
            />
        </>
    )
}
