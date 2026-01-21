// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren, ReactElement} from "react"
import {
    DatagridConfigurable,
    List,
    NumberField,
    ReferenceField,
    TextField,
    TextInput,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {Typography} from "@mui/material"
import {generateRowClickHandler} from "../../services/RowClickService"
import {useTenantStore} from "../../providers/TenantContextProvider"

const OMIT_FIELDS = ["id"]

const Filters: Array<ReactElement> = [
    <TextInput label="Name" source="name" key={0} />,
    <TextInput label="Media Type" source="media_type" key={1} />,
    <TextInput label="ID" source="id" key={2} />,
    <TextInput label="Election Event ID" source="election_event_id" key={3} />,
]

export interface ListDocumentProps {
    aside?: ReactElement
}

export const ListDocument: React.FC<ListDocumentProps & PropsWithChildren> = ({aside}) => {
    const [tenantId] = useTenantStore()
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)

    const rowClickHandler = generateRowClickHandler(["election_event_id"], true)

    return (
        <>
            <Typography variant="h5">Documents</Typography>
            <List
                actions={
                    <ListActions open={openDrawer} setOpen={setOpenDrawer} withFilter={true} />
                }
                filter={{tenant_id: tenantId || undefined}}
                aside={aside || <div>hey</div>}
                filters={Filters}
            >
                <DatagridConfigurable rowClick={rowClickHandler} omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="name" />
                    <TextField source="media_type" />
                    <NumberField source="size" />
                    <ReferenceField
                        source="election_event_id"
                        reference="sequent_backend_election_event"
                    >
                        <TextField source="name" />
                    </ReferenceField>
                </DatagridConfigurable>
            </List>
        </>
    )
}
