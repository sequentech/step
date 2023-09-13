// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    ReferenceField,
    ReferenceManyField,
    TextInput,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {useTenantStore} from "../../components/CustomMenu"
import {Typography} from "@mui/material"
import {ChipList} from "../../components/ChipList"
import {generateRowClickHandler} from "../../services/RowClickService"

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

export const ListArea: React.FC<ListAreaProps> = ({aside}) => {
    const [tenantId] = useTenantStore()

    const rowClickHandler = generateRowClickHandler(["election_event_id"])

    return (
        <>
            <Typography variant="h5">Areas</Typography>
            <List
                actions={<ListActions withFilter={true} />}
                sx={{flexGrow: 2}}
                aside={aside}
                filter={{
                    tenant_id: tenantId || undefined,
                }}
                filters={Filters}
            >
                <DatagridConfigurable rowClick={rowClickHandler} omit={OMIT_FIELDS}>
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
                </DatagridConfigurable>
            </List>
        </>
    )
}
