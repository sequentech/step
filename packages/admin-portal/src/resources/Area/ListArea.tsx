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
    RecordContext,
    useRecordContext,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {useTenantStore} from "../../components/CustomMenu"
import {Drawer, IconButton, Typography} from "@mui/material"
import CheckIcon from "@mui/icons-material/Check"
import CancelIcon from "@mui/icons-material/Cancel"
import {ChipList} from "../../components/ChipList"
import {EditArea} from "./EditArea"
import {CreateArea} from "./CreateArea"
import {useLocation, useParams, useRoutes} from "react-router"
import {Sequent_Backend_Election_Event} from "../../gql/graphql"

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
    const record = useRecordContext<Sequent_Backend_Election_Event>()

    const [tenantId] = useTenantStore()
    const params = useParams()
    const location = useLocation()

    const [open, setOpen] = React.useState(false)
    const [closeDrawer, setCloseDrawer] = React.useState("")
    const [recordId, setRecordId] = React.useState<Identifier | undefined>(undefined)

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

    const handleCloseCreateDrawer = () => {
        setRecordId(undefined)
        setCloseDrawer(new Date().toISOString())
    }

    const handleCloseEditDrawer = () => {
        setOpen(false)
        setTimeout(() => {
            setRecordId(undefined)
        }, 400)
    }

    const ActionsColumn = ({source}: {source: Identifier | undefined}) => {
        const record = useRecordContext()

        return (
            <>
                <IconButton onClick={(e) => console.log(record.id)}>
                    <CheckIcon color="action" />
                </IconButton>
                <IconButton onClick={(e) => console.log(record.id)}>
                    <CancelIcon color="action" />
                </IconButton>
            </>
        )
    }

    return (
        <>
            <Typography variant="h5">Areas</Typography>
            <List
                resource="sequent_backend_area"
                actions={
                    <ListActions
                        withImport={false}
                        closeDrawer={closeDrawer}
                        Component={<CreateArea record={record} close={handleCloseCreateDrawer} />}
                    />
                }
                sx={{flexGrow: 2}}
                filter={{
                    tenant_id: tenantId || undefined,
                }}
                filters={Filters}
            >
                <DatagridConfigurable  omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="name" />
                    <TextField source="description" />
                    <TextField source="type" />
                    <ReferenceField
                        label="Election Event"
                        reference="sequent_backend_election_event"
                        source="election_event_id"
                    >
                        <TextField source="name" />
                    </ReferenceField>
                    <ReferenceManyField
                        label="Area Contests"
                        reference="sequent_backend_area_contest"
                        target="area_id"
                    >
                        <ChipList
                            source="sequent_backend_area_contest"
                            filterFields={["election_event_id", "area_id"]}
                        />
                    </ReferenceManyField>
                    <ActionsColumn source={"1"}/>
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
                <EditArea id={recordId} close={handleCloseEditDrawer} />
            </Drawer>
        </>
    )
}
