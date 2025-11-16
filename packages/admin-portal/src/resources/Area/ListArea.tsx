// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useEffect, useState} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    TextInput,
    Identifier,
    useRecordContext,
    useDelete,
    WrapperField,
    FunctionField,
    useRefresh,
    useNotify,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {Box, Button, Drawer, Typography} from "@mui/material"
import {ImportAreasMutation, Sequent_Backend_Election_Event} from "../../gql/graphql"
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
import {ImportDataDrawer} from "@/components/election-event/import-data/ImportDataDrawer"
import {useMutation} from "@apollo/client"
import {IMPORT_AREAS} from "@/queries/ImportAreas"
import {styled} from "@mui/material/styles"
import {UPSERT_AREAS} from "@/queries/UpsertAreas"
import {ResetFilters} from "@/components/ResetFilters"
import {useAreaPermissions} from "./useAreaPermissions"
import {UpsertArea} from "./UpsertArea"
import {EElectionEventWeightedVotingPolicy} from "@sequentech/ui-core"

const ActionsBox = styled(Box)`
    display: flex;
    flex-direction: row;
    gap: 8px;
`

const OMIT_FIELDS = ["id", "ballot_eml"]

const Filters: Array<ReactElement> = [
    <TextInput label="Name" source="name" key={0} />,
    <TextInput label="Description" source="description" key={1} />,
    <TextInput label="ID" source="id" key={2} />,
    <TextInput label="Type" source="type" key={3} />,
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

    const [open, setOpen] = useState(false)
    const [openCreate, setOpenCreate] = useState(false)
    const [openDeleteModal, setOpenDeleteModal] = useState(false)
    const [deleteId, setDeleteId] = useState<Identifier | undefined>()
    const [openDrawer, setOpenDrawer] = useState<boolean>(false)
    const [recordId, setRecordId] = useState<Identifier | undefined>(undefined)
    const [openImportDrawer, setOpenImportDrawer] = useState(false)
    const [openUpsertDrawer, setOpenUpsertDrawer] = useState(false)

    const {
        canCreateArea,
        canEditArea,
        canReadArea,
        canDeleteArea,
        canImportArea,
        canUpsertArea,
        showAreaColumns,
        showAreaFilters,
    } = useAreaPermissions()

    const weightedVotingForAreas =
        record?.presentation?.weighted_voting_policy ===
        EElectionEventWeightedVotingPolicy.AREAS_WEIGHTED_VOTING

    const [importAreas] = useMutation<ImportAreasMutation>(IMPORT_AREAS, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.AREA_IMPORT,
            },
        },
    })
    const [upsertAreas] = useMutation<ImportAreasMutation>(UPSERT_AREAS, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.AREA_UPSERT,
            },
        },
    })

    // const authContext = useContext(AuthContext)
    // const canView = authContext.isAuthorized(true, tenantId, IPermissions.AREA_READ)
    // const canCreate = authContext.isAuthorized(true, tenantId, IPermissions.AREA_WRITE)

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
            {canCreateArea && (
                <>
                    <ActionsBox>
                        <Button onClick={createAction} className="area-add-button">
                            <IconButton icon={faPlus as any} fontSize="24px" />
                            {t("areas.empty.action")}
                        </Button>
                        <Button onClick={() => setOpenImportDrawer(true)}>
                            <IconButton icon={faPlus as any} fontSize="24px" />
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
                electionEventId: record?.id,
                sha256,
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
                electionEventId: record?.id,
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
        {
            icon: <EditIcon className="edit-area-icon" />,
            action: editAction,
            showAction: () => canEditArea,
        },
        {
            icon: <DeleteIcon className="delete-area-icon" />,
            action: deleteAction,
            showAction: () => canDeleteArea,
        },
    ]

    // check if data array is empty
    //const {data, isLoading} = listContext

    if (!canReadArea) {
        return <Empty />
    }

    return (
        <>
            {
                <>
                    {
                        <List
                            resource="sequent_backend_area"
                            actions={
                                <ListActions
                                    withColumns={showAreaColumns}
                                    withFilter={showAreaFilters}
                                    withImport={canImportArea}
                                    doImport={() => setOpenImportDrawer(true)}
                                    withExport={false}
                                    open={openDrawer}
                                    setOpen={setOpenDrawer}
                                    Component={
                                        <UpsertArea
                                            record={record}
                                            electionEventId={id}
                                            close={handleCloseCreateDrawer}
                                            weightedVotingForAreas={weightedVotingForAreas}
                                        />
                                    }
                                    withComponent={canCreateArea}
                                    extraActions={[
                                        canUpsertArea ? (
                                            <Button
                                                onClick={() => setOpenUpsertDrawer(true)}
                                                key="upsert"
                                            >
                                                {t("electionEventScreen.importAreas.upsert")}
                                            </Button>
                                        ) : (
                                            <></>
                                        ),
                                    ]}
                                />
                            }
                            empty={<Empty />}
                            sx={{flexGrow: 2}}
                            storeKey={false}
                            filters={Filters}
                            filter={{
                                tenant_id: tenantId || undefined,
                                election_event_id: record?.id || undefined,
                            }}
                            filterDefaultValues={{}}
                            disableSyncWithLocation
                        >
                            <ResetFilters />
                            <DatagridConfigurable omit={OMIT_FIELDS} rowClick={false}>
                                <TextField source="id" />
                                <TextField source="name" className="area-name" />
                                <TextField source="description" className="area-description" />

                                <FunctionField
                                    label={String(t("areas.sequent_backend_area_contest"))}
                                    render={(record: any) => <AreaContestItems record={record} />}
                                />
                                {weightedVotingForAreas && (
                                    <TextField source="annotations.weight" label="Weight" />
                                )}
                                <WrapperField source="actions" label="Actions">
                                    <ActionsColumn actions={actions} />
                                </WrapperField>
                            </DatagridConfigurable>
                        </List>
                    }
                </>
            }
            <Drawer
                anchor="right"
                open={open}
                onClose={handleCloseEditDrawer}
                PaperProps={{
                    sx: {width: "40%"},
                }}
            >
                <UpsertArea
                    record={record}
                    id={recordId}
                    electionEventId={id}
                    close={handleCloseEditDrawer}
                    weightedVotingForAreas={weightedVotingForAreas}
                />
            </Drawer>
            <Drawer
                anchor="right"
                open={openCreate}
                onClose={handleCloseCreateDrawer}
                PaperProps={{
                    sx: {width: "40%"},
                }}
            >
                <UpsertArea
                    record={record}
                    electionEventId={id}
                    close={handleCloseCreateDrawer}
                    weightedVotingForAreas={weightedVotingForAreas}
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
