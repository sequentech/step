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
    Empty,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {Button, Drawer, Tooltip, Typography, dialogActionsClasses} from "@mui/material"
import {EditTallySheet} from "./EditTallySheet"
import {CreateTallySheet} from "./CreateTallySheet"
import {
    PublishTallySheetMutation,
    Sequent_Backend_Contest,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Sheet,
} from "../../gql/graphql"
import {Dialog, isUndefined} from "@sequentech/ui-essentials"
import {Action, ActionsColumn} from "../../components/ActionButons"
import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "../../providers/TenantContextProvider"
import {useParams} from "react-router"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
// import {TallySheetContestItems} from "@/components/TallySheetContestItems"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {IconButton} from "@sequentech/ui-essentials"
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

const OMIT_FIELDS = ["id", "ballot_eml"]

const Filters: Array<ReactElement> = [
    <TextInput label="Name" source="name" key={0} />,
    <TextInput label="Description" source="description" key={1} />,
    <TextInput label="ID" source="id" key={2} />,
    <TextInput label="Type" source="type" key={3} />,
    <TextInput source="election_event_id" key={3} />,
]

type TTallySheetList = {
    contest: Sequent_Backend_Contest
    doAction: (action: number, id?: Identifier) => void
    reload: string | null
}

export const ListTallySheet: React.FC<TTallySheetList> = (props) => {
    const {contest, doAction, reload} = props

    const {t} = useTranslation()
    const refresh = useRefresh()
    const {globalSettings} = useContext(SettingsContext)

    const [deleteOne, {isLoading, error}] = useDelete()

    const [open, setOpen] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [openUnpublishDialog, setOpenUnpublishDialog] = React.useState(false)
    const [deleteId, setDeleteId] = React.useState<Identifier | undefined>()
    const [recordId] = React.useState<Identifier | undefined>(undefined)
    const [publishTallySheet] = useMutation<PublishTallySheetMutation>(PUBLISH_TALLY_SHEET)

    useEffect(() => {
        if (reload) {
            refresh()
        }
    }, [reload])

    useEffect(() => {
        if (recordId) {
            setOpen(true)
        }
    }, [recordId])

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t("tallysheet.empty.header")}
            </Typography>
            {/* {canWrite && ( */}
            <>
                <Button onClick={onClickPublishTallySheet}>Felix TEST puedes borrarlo</Button>
                <Button onClick={createAction}>
                    <IconButton icon={faPlus} fontSize="24px" />
                    {t("tallysheet.empty.action")}
                </Button>
                <Typography variant="body1" paragraph>
                    {t("common.resources.noResult.askCreate")}
                </Typography>
            </>
            {/* )} */}
        </ResourceListStyles.EmptyBox>
    )

    // if (!canRead) {
    //     return <Empty />
    // }

    const onClickPublishTallySheet = async () => {
        const {data, errors} = await publishTallySheet({
            variables: {
                electionEventId: "c83861cd-a912-4172-a8f5-fc9a35c8fb55",
                tallySheetId: "faef77c8-6905-439d-8b78-80dd8a76ca74",
            },
        })
        if (data && !data?.publish_tally_sheet?.tally_sheet_id) {
            // (unpublished) tally sheet not found, probably it's already published
        }
        if (errors) {
            // add error notification
        }
    }

    const createAction = () => {
        doAction(WizardSteps.Start)
        console.log("createAction")
    }

    const editAction = (id: Identifier) => {
        doAction(WizardSteps.Edit, id)
    }

    const viewAction = (id: Identifier) => {
        console.log("viewAction", id)
        doAction(WizardSteps.Confirm, id)
    }

    const publishAction = (id: Identifier) => {
        console.log("publishAction", id)
        doAction(WizardSteps.Confirm, id)
    }

    const unpublishAction = (id: Identifier) => {
        console.log("unpublishAction", id)
        setDeleteId(id)
        setOpenUnpublishDialog(true)
    }

    const deleteAction = (id: Identifier) => {
        // deleteOne("sequent_backend_TallySheet", {id})
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

    const confirmUnpublishAction = () => {
        // deleteOne(
        //     "sequent_backend_tally_sheet",
        //     {id: deleteId},
        //     {
        //         onSuccess() {
        //             refresh()
        //         },
        //     }
        // )
        setDeleteId(undefined)
    }

    const actions: Action[] = [
        {icon: <EditIcon />, action: editAction},
        {icon: <VisibilityIcon />, action: viewAction},
        {
            icon: (
                <Tooltip title={t("tallysheet.common.publish")}>
                    <PublishedWithChangesIcon />
                </Tooltip>
            ),
            action: publishAction,
        },
        {
            icon: (
                <Tooltip title={t("tallysheet.common.unpublish")}>
                    <UnpublishedIcon />
                </Tooltip>
            ),
            action: unpublishAction,
        },
        {icon: <DeleteIcon />, action: deleteAction},
    ]

    return (
        <>
            <List
                queryOptions={{
                    refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
                }}
                resource="sequent_backend_tally_sheet"
                actions={
                    <ListActions
                        withImport={false}
                        withExport={false}
                        extraActions={[
                            <Button key="add" onClick={createAction}>
                                <Add />
                                {t("tallysheet.empty.add")}
                            </Button>,
                        ]}
                    />
                }
                sx={{flexGrow: 2}}
                filter={{
                    tenant_id: contest.tenant_id || undefined,
                    election_event_id: contest.election_event_id || undefined,
                    contest_id: contest.id || undefined,
                }}
                filters={Filters}
                empty={<Empty />}
            >
                <DatagridConfigurable omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="channel" />

                    <FunctionField
                        label={t("tallysheet.table.contest")}
                        render={(record: any) => <ContestItem record={contest.id} />}
                    />

                    <FunctionField
                        label={t("tallysheet.table.area")}
                        render={(record: any) => <AreaItem record={record.area_id} />}
                    />

                    <FunctionField
                        label={t("tallysheet.table.published")}
                        render={(record: any) => record.published_at ? <CheckCircleOutlineIcon /> : null}
                    />

                    <WrapperField source="actions" label="Actions">
                        <ActionsColumn actions={actions} />
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
                        confirmUnpublishAction()
                    }
                    setOpenUnpublishDialog(false)
                }}
            >
                {t("tallysheet.common.warning")}
            </Dialog>
        </>
    )
}
