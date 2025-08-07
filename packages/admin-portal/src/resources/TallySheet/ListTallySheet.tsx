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
    useDelete,
    WrapperField,
    FunctionField,
    useRefresh,
    useNotify,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {Button, Tooltip, Typography} from "@mui/material"
import {
    PublishTallySheetMutation,
    Sequent_Backend_Contest,
    Sequent_Backend_Election,
    Sequent_Backend_Tally_Sheet,
} from "../../gql/graphql"
import {Dialog, IconButton} from "@sequentech/ui-essentials"
import {Action, ActionsColumn} from "../../components/ActionButons"
import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import {useTranslation} from "react-i18next"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import VisibilityIcon from "@mui/icons-material/Visibility"
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline"
import UnpublishedIcon from "@mui/icons-material/Unpublished"
import PublishedWithChangesIcon from "@mui/icons-material/PublishedWithChanges"
import {WizardSteps} from "./TallySheetWizard"
import {useMutation} from "@apollo/client"
import {PUBLISH_TALLY_SHEET} from "@/queries/PublishTallySheet"
import {ContestItem} from "@/components/ContestItem"
import {AreaItem} from "@/components/AreaItem"
import {Add} from "@mui/icons-material"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {AuthContext} from "@/providers/AuthContextProvider"

const OMIT_FIELDS = ["id", "ballot_eml"]

const Filters: Array<ReactElement> = [
    <TextInput label="Area" source="area_id" key={0} />,
    <TextInput label="Contest" source="contest" key={1} />,
    <TextInput label="ID" source="id" key={2} />,
    <TextInput label="Published" source="published_at" key={3} />,
]

interface TTallySheetList {
    election: Sequent_Backend_Election
    doAction: (action: number, id?: Identifier) => void
    reload: string | null
}

export const ListTallySheet: React.FC<TTallySheetList> = (props) => {
    const {election: election, doAction, reload} = props

    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const refresh = useRefresh()
    const {globalSettings} = useContext(SettingsContext)
    const notify = useNotify()

    const [deleteOne] = useDelete()

    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [openUnpublishDialog, setOpenUnpublishDialog] = React.useState(false)
    const [openPublishDialog, setOpenPublishDialog] = React.useState(false)
    const [deleteId, setDeleteId] = React.useState<Identifier | undefined>()
    const [publishTallySheet] = useMutation<PublishTallySheetMutation>(PUBLISH_TALLY_SHEET)
    const [publish, setPublish] = React.useState(false)

    const authContext = useContext(AuthContext)
    const canCreate = authContext.isAuthorized(true, tenantId, IPermissions.TALLY_SHEET_CREATE)
    const canView = authContext.isAuthorized(true, tenantId, IPermissions.TALLY_SHEET_VIEW)
    const canPublish = authContext.isAuthorized(true, tenantId, IPermissions.TALLY_SHEET_PUBLISH)
    const canDelete = authContext.isAuthorized(true, tenantId, IPermissions.TALLY_SHEET_DELETE)

    /// For the tally sheets table: The columns should include the approved version and the latest version instead of "Published".
    /// This table is at election level. It should list the ballot boxes (area, contest, channel).

    /// For the versions Sreen table - List all tally sheet versions for that box, which means related to the same (area, contest, channel).
    // get_tally_sheet_versions variables(area, election_id, contest_id, channel)

    useEffect(() => {
        localStorage.removeItem("tallySheetData")
    }, [])

    useEffect(() => {
        if (reload) {
            refresh()
        }
    }, [reload, refresh])

    const createAction = () => {
        localStorage.removeItem("tallySheetData")
        doAction(WizardSteps.Start)
    }

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t("tallysheet.empty.header")}
            </Typography>
            {canCreate && (
                <>
                    <Button onClick={createAction}>
                        <IconButton icon={faPlus} fontSize="24px" />
                        {t("tallysheet.empty.action")}
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

    const editAction = (id: Identifier) => {
        doAction(WizardSteps.Edit, id)
    }

    const viewAction = (id: Identifier) => {
        doAction(WizardSteps.View, id)
    }

    const publishAction = (id: Identifier) => {
        setDeleteId(id)
        setOpenPublishDialog(true)
    }

    const unpublishAction = (id: Identifier) => {
        setDeleteId(id)
        setOpenUnpublishDialog(true)
    }

    const deleteAction = (id: Identifier) => {
        setOpenDeleteModal(true)
        setDeleteId(id)
    }

    const confirmDeleteAction = () => {
        deleteOne(
            "sequent_backend_tally_sheet",
            {id: deleteId},
            {
                onSuccess() {
                    refresh()
                },
            }
        )
        setDeleteId(undefined)
    }

    const confirmPublishAction = async (isPublished: boolean) => {
        const {data, errors} = await publishTallySheet({
            variables: {
                electionEventId: election.election_event_id,
                tallySheetId: deleteId,
                publish: isPublished,
            },
        })
        if (data && !data?.publish_tally_sheet?.tally_sheet_id) {
            console.log("(unpublished) tally sheet not found, probably it's already published")
        }
        if (errors) {
            // add error notification
            notify(t("tallysheet.message.publishError"), {type: "error"})
        } else {
            notify(t("tallysheet.message.publishSuccess"), {type: "success"})
        }
        setDeleteId(undefined)
    }

    const actions: (record: Sequent_Backend_Tally_Sheet) => Action[] = (record) => [
        {icon: <EditIcon />, action: editAction, showAction: () => canCreate},
        {icon: <VisibilityIcon />, action: viewAction, showAction: () => canView},
        {
            icon: (
                <Tooltip title={t("tallysheet.common.publish")}>
                    <PublishedWithChangesIcon />
                </Tooltip>
            ),
            action: publishAction,
            showAction: () => canPublish && record.published_at === null,
        },
        {
            icon: (
                <Tooltip title={t("tallysheet.common.unpublish")}>
                    <UnpublishedIcon />
                </Tooltip>
            ),
            action: unpublishAction,
            showAction: () => canPublish && record.published_at !== null,
        },
        {icon: <DeleteIcon />, action: deleteAction, showAction: () => canDelete},
    ]

    return (
        <>
            {/* <CustomApolloContextProvider role="tally-sheet-view">
                <ActionPublish publish={publish} setPublish={setPublish} />
            </CustomApolloContextProvider> */}
            <List
                queryOptions={{
                    refetchInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
                }}
                resource="sequent_backend_tally_sheet"
                actions={
                    <ListActions
                        withImport={false}
                        withExport={false}
                        extraActions={[
                            <Button key={0} onClick={createAction}>
                                <Add />
                                {t("tallysheet.empty.add")}
                            </Button>,
                        ]}
                    />
                }
                sx={{flexGrow: 2}}
                filter={{
                    tenant_id: election.tenant_id || undefined,
                    election_event_id: election.election_event_id || undefined,
                    election_id: election.id || undefined,
                    deleted_at: {
                        format: "hasura-raw-query",
                        value: {_is_null: true},
                    },
                }}
                filters={Filters}
                empty={<Empty />}
            >
                <DatagridConfigurable omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="channel" />

                    <FunctionField
                        label={t("tallysheet.table.contest")}
                        render={(record: any) => <ContestItem record={election.id} />}
                    />

                    <FunctionField
                        label={t("tallysheet.table.area")}
                        render={(record: Sequent_Backend_Tally_Sheet) => (
                            <AreaItem record={record.area_id} />
                        )}
                    />

                    <FunctionField
                        label={t("tallysheet.table.published")}
                        render={(record: any) =>
                            record.published_at ? <CheckCircleOutlineIcon color="success" /> : null
                        }
                    />

                    <WrapperField source="actions" label="Actions">
                        <FunctionField
                            label={t("tallysheet.table.area")}
                            render={(record: Sequent_Backend_Tally_Sheet) => (
                                <ActionsColumn actions={actions(record)} />
                            )}
                        />
                        {/* <ActionsColumn actions={actions} /> */}
                    </WrapperField>
                </DatagridConfigurable>
            </List>

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

            <Dialog
                variant="warning"
                open={openUnpublishDialog}
                ok={t("tallysheet.common.unpublish")}
                cancel={t("common.label.cancel")}
                title={t("tallysheet.common.unpublish")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmPublishAction(false)
                    }
                    setOpenUnpublishDialog(false)
                }}
            >
                {t("tallysheet.common.warningUnPublish")}
            </Dialog>

            <Dialog
                variant="info"
                open={openPublishDialog}
                ok={t("tallysheet.common.publish")}
                cancel={t("common.label.cancel")}
                title={t("tallysheet.common.publish")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmPublishAction(true)
                    }
                    setOpenPublishDialog(false)
                }}
            >
                {t("tallysheet.common.warningPublish")}
            </Dialog>
        </>
    )
}
