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
    Empty,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {Button, Drawer, Typography, dialogActionsClasses} from "@mui/material"
import {EditTallySheet} from "./EditTallySheet"
import {CreateTallySheet} from "./CreateTallySheet"
import {Sequent_Backend_Contest, Sequent_Backend_Election_Event, Sequent_Backend_Tally_Session, Sequent_Backend_Tally_Sheet} from "../../gql/graphql"
import {Dialog} from "@sequentech/ui-essentials"
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
import PublishIcon from "@mui/icons-material/Publish"
import UnpublishedIcon from "@mui/icons-material/Unpublished"
import { WizardSteps } from './TallySheetWizard'

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
}

export const ListTallySheet: React.FC<TTallySheetList> = (props) => {
    const {contest, doAction} = props

    const {t} = useTranslation()
    const {id} = useParams()
    const refresh = useRefresh()

    const record = useRecordContext<Sequent_Backend_Tally_Sheet>()

    const [tenantId] = useTenantStore()
    
    const [eventId, setEventId] = React.useState<Identifier | undefined>()
    const [electionId, setElectionId] = React.useState<Identifier | undefined>()
    
    const [deleteOne, {isLoading, error}] = useDelete()

    const [open, setOpen] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [deleteId, setDeleteId] = React.useState<Identifier | undefined>()
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)
    const [recordId, setRecordId] = React.useState<Identifier | undefined>(undefined)

    // const rowClickHandler = generateRowClickHandler(["election_event_id"])
    const rowClickHandler = (id: Identifier, resource: string, record: RaRecord) => {
        setRecordId(id)
        return ""
    }

    useEffect(() => {
        if (contest) {
            setEventId(contest.election_event_id)
            setElectionId(contest.election_id)
        }
    }, [contest])

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

    const createAction = () => {
        doAction(WizardSteps.Start)
        console.log("createAction")
    }

    const editAction = (id: Identifier) => {
        doAction(WizardSteps.Edit, id)
    }

    const viewAction = (id: Identifier) => {
        console.log("viewAction", id)
        doAction(WizardSteps.View, id)
    }

    const publishAction = (id: Identifier) => {
        console.log("publishAction", id)
        doAction(WizardSteps.Confirm, id)
    }

    const unpublishAction = (id: Identifier) => {
        console.log("unpublishAction", id)
        // setRecordId(id)
    }

    const deleteAction = (id: Identifier) => {
        // deleteOne("sequent_backend_TallySheet", {id})
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
            }
        )
        setDeleteId(undefined)
    }

    const actions: Action[] = [
        {icon: <EditIcon />, action: editAction},
        {icon: <VisibilityIcon />, action: viewAction},
        {icon: <PublishIcon />, action: publishAction},
        {icon: <UnpublishedIcon />, action: unpublishAction},
        {icon: <DeleteIcon />, action: deleteAction},
    ]

    return (
        <>
            <List
                resource="sequent_backend_area"
                actions={
                    <ListActions
                        withImport={false}
                        // open={openDrawer}
                        // setOpen={setOpenDrawer}
                        // Component={
                        //     <CreateTallySheet record={record} close={handleCloseCreateDrawer} />
                        // }
                    />
                }
                sx={{flexGrow: 2}}
                filter={{
                    tenant_id: tenantId || undefined,
                    election_event_id: record?.id || undefined,
                }}
                filters={Filters}
                empty={<Empty />}
            >
                <DatagridConfigurable omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="name" />
                    <TextField source="description" />

                    {/* <FunctionField
                        label={t("TallySheets.sequent_backend_TallySheet_contest")}
                        render={(record: any) => <TallySheetContestItems record={record} />}
                    /> */}

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
        </>
    )
}
