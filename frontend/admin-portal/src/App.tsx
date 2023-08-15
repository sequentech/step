// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {
    Admin,
    DataProvider,
    Resource,
    CustomRoutes,
} from "react-admin"
import buildHasuraProvider from "ra-data-hasura"
import {apolloClient} from "./services/ApolloService"
import {Route} from "react-router-dom"
import {UserAndRoles} from "./screens/UserAndRoles"
import {Settings} from "./screens/Settings"
import {Messages} from "./screens/Messages"
import { CustomLayout } from "./components/CustomLayout"
import { ElectionEventList } from "./resources/ElectionEvent/ElectionEventList"
import { CreateElectionList } from "./resources/ElectionEvent/CreateElectionEvent"
import { EditElectionList } from "./resources/ElectionEvent/EditElectionEvent"
import { EditElection } from "./resources/Election/EditElection"
import { ElectionList } from "./resources/Election/ListElection"
import { EditContest } from "./resources/Contest/EditContest"
import { ContestList } from "./resources/Contest/ListContest"
import { CreateElection } from "./resources/Election/CreateElection"
import { CreateContest } from "./resources/Contest/CreateContest"

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
        <Admin dataProvider={dataProvider || undefined} layout={CustomLayout}>
            <CustomRoutes>
                <Route path="/user-roles" element={<UserAndRoles />} />
                <Route path="/settings" element={<Settings />} />
                <Route path="/messages" element={<Messages />} />
            </CustomRoutes>
            <Resource
                name="sequent_backend_election_event"
                list={ElectionEventList}
                create={CreateElectionList}
                edit={EditElectionList}
                options={{label: "Election Events"}}
            />
            <Resource
                name="sequent_backend_election"
                edit={EditElection}
                list={ElectionList}
                create={CreateElection}
                options={{label: "Elections"}}
            />
            <Resource
                name="sequent_backend_contest"
                edit={EditContest}
                list={ContestList}
                create={CreateContest}
                options={{label: "Contests"}}
            />
        </Admin>
    )
}

export default App
