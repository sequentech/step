// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState, PropsWithChildren} from "react"
import {
    Admin,
    DataProvider,
    List,
    Resource,
    Datagrid,
    TextField,
    ReferenceManyField,
    useListContext,
} from "react-admin"
import buildHasuraProvider from "ra-data-hasura"
import {apolloClient} from "./services/ApolloService"
import {Chip} from "@mui/material"
import {Sequent_Backend_Election} from "./gql/graphql"

const ElectionList: React.FC = () => {
    const {data} = useListContext<{id: string; name: string}>()
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
    <List>
        <Datagrid>
            <TextField source="name" />
            <ReferenceManyField
                label="Elections"
                reference="sequent_backend_election"
                target="election_event_id"
            >
                <ElectionList />
            </ReferenceManyField>
        </Datagrid>
    </List>
)

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
                options={{label: "Election Events"}}
            />
        </Admin>
    )
}

export default App
