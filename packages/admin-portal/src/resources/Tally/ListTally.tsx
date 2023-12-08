// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useEffect} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    ReferenceField,
    ReferenceManyField,
    TextInput,
    Identifier,
    RaRecord,
    useRecordContext,
    useDelete,
    WrapperField,
    Datagrid,
    FunctionField,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {Drawer} from "@mui/material"
import {ChipList} from "../../components/ChipList"
// import {EditArea} from "./EditArea"
import {CreateTally} from "./CreateTally"
import {Sequent_Backend_Election_Event, Sequent_Backend_Tally_Session} from "../../gql/graphql"
import {Dialog} from "@sequentech/ui-essentials"
import {Action, ActionsColumn} from "../../components/ActionButons"
import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import DescriptionIcon from "@mui/icons-material/Description"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "../../providers/TenantContextProvider"
import {useParams} from "react-router"
import {AreaContestItems} from "@/components/AreaContestItems"
import ElectionHeader from '@/components/ElectionHeader'
import { EditTally } from './EditTally'
import { TrusteeItems } from '@/components/TrusteeItems'
import { useElectionEventTallyStore } from '@/providers/ElectionEventTallyProvider'
import { StatusChip } from '@/components/StatusChip'

const OMIT_FIELDS = ["id", "ballot_eml"]

const Filters: Array<ReactElement> = [
    <TextInput label="Name" source="name" key={0} />,
    <TextInput label="Description" source="description" key={1} />,
    <TextInput label="ID" source="id" key={2} />,
    <TextInput label="Type" source="type" key={3} />,
    <TextInput source="election_event_id" key={3} />,
]

export interface ListAreaProps {
    record: Sequent_Backend_Tally_Session
    aside?: ReactElement
}

export const ListTally: React.FC<ListAreaProps> = (props) => {
    const {t} = useTranslation()
    const {id} = useParams()

    const record = useRecordContext<Sequent_Backend_Election_Event>()

    const [tenantId] = useTenantStore()
    const [_, setTallyId] = useElectionEventTallyStore()
    const [deleteOne] = useDelete()

    const [open, setOpen] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [deleteId, setDeleteId] = React.useState<Identifier | undefined>()
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)
    const [recordId, setRecordId] = React.useState<Identifier | undefined>(undefined)

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

    const editDetail = (id: Identifier) => {
        setTallyId(id as string)
    }

    const confirmDeleteAction = () => {
        deleteOne("sequent_backend_area", {id: deleteId})
        setDeleteId(undefined)
    }

    const actions: Action[] = [
        {icon: <DescriptionIcon />, action: editDetail},
    ]

    return (
        <>
            <List
                resource="sequent_backend_tally_session"
                actions={
                    <ListActions
                        withColumns={false}
                        withImport={false}
                        withExport={false}
                        // withFilter={false}
                        open={openDrawer}
                        setOpen={setOpenDrawer}
                        Component={<CreateTally record={record} close={handleCloseCreateDrawer} />}
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
                <ElectionHeader title={t("electionEventScreen.tally.title")} subtitle="" />

                <DatagridConfigurable omit={OMIT_FIELDS}>
                    <TextField source="tenant_id" />
                    <TextField source="election_event_id" />

                    <FunctionField
                        label={t("electionEventScreen.tally.trustees")}
                        render={(record: RaRecord<Identifier>) => <TrusteeItems record={record} />}
                    />

                    <FunctionField
                        label={t("electionEventScreen.tally.electionNumber")}
                        render={(record: RaRecord<Identifier>) => record?.election_ids?.length || 0}
                    />

                    <FunctionField
                        label={t("electionEventScreen.tally.status")}
                        render={(record: RaRecord<Identifier>) => (
                            <StatusChip status={record.execution_status} />
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
                <EditTally id={recordId} electionEventId={id} close={handleCloseEditDrawer} />
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
