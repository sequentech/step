// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement} from "react"
import {
    DatagridConfigurable,
    List,
    BooleanField,
    TextField,
    TextInput,
    BooleanInput,
    ReferenceField,
    FunctionField,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {Typography} from "@mui/material"
import {generateRowClickHandler} from "../../services/RowClickService"
import {useTenantStore} from "../../providers/TenantContextProvider"
import {electionSearchQuery, electionSearchText} from "../../services/ElectionSearch"
import {usePresentationName} from "../../hooks/usePresentationName"

const OMIT_FIELDS = ["id", "type", "is_public"]

const Filters: Array<ReactElement> = [
    <TextInput
        label="Name"
        source="_or"
        key={0}
        parse={electionSearchQuery}
        format={electionSearchText}
    />,
    <TextInput label="Description" source="description" key={1} />,
    <TextInput label="ID" source="id" key={2} />,
    <BooleanInput label="Is Public" source="is_public" key={3} />,
    <TextInput source="election_event_id" key={4} />,
    <TextInput source="contest_id" key={5} />,
]

export interface ListCandidateProps {
    aside?: ReactElement
}

export const ListCandidate: React.FC<ListCandidateProps> = ({aside}) => {
    const [tenantId] = useTenantStore()
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)
    const presentationName = usePresentationName()

    const rowClickHandler = generateRowClickHandler(["election_event_id", "contest_id"])

    return (
        <>
            <Typography variant="h5">Candidates</Typography>
            <List
                actions={<ListActions open={openDrawer} setOpen={setOpenDrawer} />}
                sx={{flexGrow: 2}}
                filter={{
                    tenant_id: tenantId || undefined,
                }}
                filters={Filters}
                aside={aside}
            >
                <DatagridConfigurable rowClick={rowClickHandler} omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <FunctionField label="Name" render={presentationName} />
                    <TextField source="description" />
                    <TextField source="type" />
                    <BooleanField source="is_public" />
                    <ReferenceField
                        source="election_event_id"
                        reference="sequent_backend_election_event"
                    >
                        <FunctionField render={presentationName} />
                    </ReferenceField>
                    <ReferenceField source="contest_id" reference="sequent_backend_contest">
                        <FunctionField render={presentationName} />
                    </ReferenceField>
                </DatagridConfigurable>
            </List>
        </>
    )
}
