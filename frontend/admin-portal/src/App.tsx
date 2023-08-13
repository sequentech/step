// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState, PropsWithChildren} from "react"
import {
    Admin,
    DataProvider,
    List,
    Resource,
    TopToolbar,
    TextField,
    ReferenceManyField,
    useListContext,
    SimpleForm,
    TextInput,
    Form,
    DatagridConfigurable,
    CreateButton,
    ExportButton,
    Edit,
    SelectColumnsButton,
    Create,
    ReferenceInput,
    SelectInput,
} from "react-admin"
import buildHasuraProvider from "ra-data-hasura"
import {apolloClient} from "./services/ApolloService"
import {Chip} from "@mui/material"
import {Sequent_Backend_Election} from "./gql/graphql"

const ListActions: React.FC = () => (
    <TopToolbar>
        <SelectColumnsButton />
        {/*<FilterButton/>*/}
        <CreateButton />
        <ExportButton />
    </TopToolbar>
)

const ElectionList: React.FC = () => {
    const {data} = useListContext<Sequent_Backend_Election>()
    if (!data) {
        return null
    }

    return (
        <>
            {data.map((election) => (
                <Chip label={election.name} key={election.id} />
            ))}
        </>
    )
}

const ElectionEventList: React.FC<PropsWithChildren> = ({}) => (
    <List actions={<ListActions />}>
        <DatagridConfigurable rowClick="edit" omit={["id"]}>
            <TextField source="id" />
            <TextField source="name" />
            <TextField source="description" />
            <ReferenceManyField
                label="Elections"
                reference="sequent_backend_election"
                target="election_event_id"
            >
                <ElectionList />
            </ReferenceManyField>
        </DatagridConfigurable>
    </List>
)

const ElectionListForm: React.FC = () => {
    return (
        <SimpleForm>
            <TextInput source="description" />
            <TextInput source="name" />
            <SelectInput
                source="encryption_protocol"
                choices={[
                    {id: "RSA256", name: "RSA256"}
                ]}
            />
            <ReferenceInput source="tenant_id" reference="sequent_backend_tenant">
                <SelectInput optionText="username" />
            </ReferenceInput>
        </SimpleForm>
    )
}

const EditElectionList: React.FC = () => {
    return (
        <Edit>
            <ElectionListForm />
        </Edit>
    )
}

const CreateElectionList: React.FC = () => {
    return (
        <Create>
            <ElectionListForm />
        </Create>
    )
}

const App = () => {
    const [dataProvider, setDataProvider] = useState<DataProvider | null>(null)

    useEffect(() => {
        const buildDataProvider = async () => {
            const dataProvider = await buildHasuraProvider({
                client: apolloClient as any,
            })
            setDataProvider(() => dataProvider)
        }
        buildDataProvider()
    }, [])

    return (
        <Admin dataProvider={dataProvider || undefined}>
            <Resource
                name="sequent_backend_election_event"
                list={ElectionEventList}
                create={CreateElectionList}
                edit={EditElectionList}
                options={{label: "Election Events"}}
            />
        </Admin>
    )
}

export default App
