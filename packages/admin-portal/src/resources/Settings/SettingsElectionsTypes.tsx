import React, {ReactElement} from "react"

import {
    BooleanField,
    DatagridConfigurable,
    List,
    ReferenceManyField,
    TextField,
    TextInput,
} from "react-admin"

import {ListActions} from "../../components/ListActions"

//TODO: Remove Create Election List Component
import {ChipList} from "../../components/ChipList"

import {CreateElectionList} from "../ElectionEvent/CreateElectionEvent"

const OMIT_FIELDS = ["id", "sequent_backend_area", "is_archived", "is_audit", "public_key"]
const Filters: Array<ReactElement> = [<TextInput label="Name" source="name" key={0} />]

export const SettingsElectionsTypes: React.FC<void> = () => {
    return (
        <List
            filters={Filters}
            actions={<ListActions custom withFilter Component={<CreateElectionList />} />}
        >
            <DatagridConfigurable rowClick="edit" omit={OMIT_FIELDS}>
                <TextField source="id" />
                <TextField source="name" />
                <TextField source="description" />
                <BooleanField source="is_archived" />
                <BooleanField source="is_audit" />
                <TextField source="public_key" />
                <ReferenceManyField
                    label="Elections"
                    reference="sequent_backend_election"
                    target="election_event_id"
                >
                    <ChipList
                        source="sequent_backend_election"
                        filterFields={["election_event_id"]}
                    />
                </ReferenceManyField>
                <ReferenceManyField
                    label="Areas"
                    reference="sequent_backend_area"
                    target="election_event_id"
                >
                    <ChipList source="sequent_backend_area" filterFields={["election_event_id"]} />
                </ReferenceManyField>
            </DatagridConfigurable>
        </List>
    )
}
