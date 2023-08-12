// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState, PropsWithChildren} from "react"
import {Admin, DataProvider, List, Resource, Datagrid, TextField} from "react-admin"
import buildHasuraProvider from "ra-data-hasura"
import { apolloClient } from "./services/ApolloService"

const ElectionList: React.FC<PropsWithChildren> = ({children}) =>
    <List>
        <Datagrid>
            <TextField source="id" />
        </Datagrid>
    </List>

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
                name="sequent_backend_election"
                list={ElectionList}
                options={{label: "Elections"}}
            />
        </Admin>
    )
}

export default App
