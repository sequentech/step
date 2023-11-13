// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState, useMemo} from "react"
import {Admin, DataProvider, Resource, CustomRoutes} from "react-admin"
import buildHasuraProvider from "ra-data-hasura"
import {customBuildQuery} from "./queries/customBuildQuery"
import {createApolloClient} from "./services/ApolloService"
import {Route} from "react-router-dom"
import {UserAndRoles} from "./screens/UserAndRoles"
import {Settings} from "./screens/Settings"
import {Messages} from "./screens/Messages"
import {CustomLayout} from "./components/CustomLayout"
import {ElectionEventList} from "./resources/ElectionEvent/ElectionEventList"
import {CreateElectionList} from "./resources/ElectionEvent/CreateElectionEvent"
import {EditElectionList} from "./resources/ElectionEvent/EditElectionEvent"
import {EditElection} from "./resources/Election/EditElection"
import {ListElection} from "./resources/Election/ListElection"
import {EditContest} from "./resources/Contest/EditContest"
import {ListContest} from "./resources/Contest/ListContest"
import {CreateElection} from "./resources/Election/CreateElection"
import {CreateContest} from "./resources/Contest/CreateContest"
import {EditCandidate} from "./resources/Candidate/EditCandidate"
import {ListCandidate} from "./resources/Candidate/ListCandidate"
import {CreateCandidate} from "./resources/Candidate/CreateCandidate"
import {EditBallotStyle} from "./resources/BallotStyle/EditBallotStyle"
import {ListBallotStyle} from "./resources/BallotStyle/ListBallotStyle"
import {CreateBallotStyle} from "./resources/BallotStyle/CreateBallotStyle"
import {CreateArea} from "./resources/Area/CreateArea"
import {ListArea} from "./resources/Area/ListArea"
import {EditArea} from "./resources/Area/EditArea"
import {EditAreaContest} from "./resources/AreaContest/EditAreaContest"
import {ListAreaContest} from "./resources/AreaContest/ListAreaContest"
import {CreateAreaContest} from "./resources/AreaContest/CreateAreaContest"
import {EditTenant} from "./resources/Tenant/EditTenant"
import {ListTenant} from "./resources/Tenant/ListTenant"
import {CreateTenant} from "./resources/Tenant/CreateTenant"
import {ShowElectionEvent} from "./resources/ElectionEvent/ShowElectionEvent"
import {ShowDocument} from "./resources/Document/ShowDocument"
import {ListDocument} from "./resources/Document/ListDocument"
import {CreateDocument} from "./resources/Document/CreateDocument"
import {EditTrustee} from "./resources/Trustee/EditTrustee"
import {ListTrustee} from "./resources/Trustee/ListTrustee"
import {CreateTrustee} from "./resources/Trustee/CreateTrustee"
import {PgAuditList} from "./resources/PgAudit/PgAuditList"
import {fullAdminTheme} from "./services/AdminTheme"
import {ApolloClient, ApolloProvider, NormalizedCacheObject} from "@apollo/client"
import {isNull} from "@sequentech/ui-essentials"
import {AuthContext} from "./providers/AuthContextProvider"

export const AppWrapper = () => {
    const [apolloClient, setApolloClient] = useState<ApolloClient<NormalizedCacheObject> | null>(
        null
    )
    const authContext = useContext(AuthContext)
    const accessToken = useMemo(authContext.getAccessToken, [
        authContext.isAuthenticated,
        authContext.getAccessToken,
    ])

    useEffect(() => {
        if (accessToken) {
            let newClient = createApolloClient()
            setApolloClient(newClient)
        }
    }, [authContext.isAuthenticated, accessToken])

    if (isNull(apolloClient)) {
        return null
    }

    return (
        <ApolloProvider client={apolloClient}>
            <App apolloClient={apolloClient} />
        </ApolloProvider>
    )
}

interface AppProps {
    apolloClient: ApolloClient<NormalizedCacheObject>
}

const App: React.FC<AppProps> = ({apolloClient}) => {
    const [dataProvider, setDataProvider] = useState<DataProvider | null>(null)

    useEffect(() => {
        const buildDataProvider = async () => {
            const options = {
                client: apolloClient as any,
                buildQuery: customBuildQuery as any,
            }
            const buildGqlQueryOverrides = {}
            const dataProviderHasura = await buildHasuraProvider(options, buildGqlQueryOverrides)
            setDataProvider(() => dataProviderHasura)
        }
        buildDataProvider()
    }, [])

    if (!dataProvider) return <p>Loading data provider...</p>

    return (
        <Admin
            dataProvider={dataProvider || undefined}
            layout={CustomLayout}
            theme={fullAdminTheme}
        >
            <CustomRoutes>
                <Route path="/user-roles" element={<UserAndRoles />} />
                <Route path="/settings" element={<Settings />} />
                <Route path="/messages" element={<Messages />} />
            </CustomRoutes>
            <Resource name="pgaudit" list={PgAuditList} options={{label: "PGAudit"}} />
            <Resource
                name="sequent_backend_election_event"
                list={ElectionEventList}
                create={CreateElectionList}
                edit={EditElectionList}
                show={ShowElectionEvent}
                options={{label: "Election Events", isMenuParent: true}}
            />
            <Resource
                name="sequent_backend_election"
                edit={EditElection}
                list={ListElection}
                create={CreateElection}
                options={{
                    label: "Elections",
                    menuParent: "sequent_backend_election_event",
                    foreignKeyFrom: "election_event_id",
                }}
            />
            <Resource
                name="sequent_backend_contest"
                edit={EditContest}
                list={ListContest}
                create={CreateContest}
                options={{
                    label: "Contests",
                    menuParent: "sequent_backend_election",
                    foreignKeyFrom: "election_id",
                }}
            />
            <Resource
                name="sequent_backend_candidate"
                edit={EditCandidate}
                list={ListCandidate}
                create={CreateCandidate}
                options={{
                    label: "Candidates",
                    menuParent: "sequent_backend_contest",
                    foreignKeyFrom: "contest_id",
                }}
            />
            <Resource
                name="sequent_backend_ballot_style"
                edit={EditBallotStyle}
                list={ListBallotStyle}
                create={CreateBallotStyle}
                options={{label: "Ballot Styles"}}
            />
            <Resource
                name="sequent_backend_area"
                edit={EditArea}
                list={ListArea}
                create={CreateArea}
                options={{label: "Area"}}
            />
            <Resource
                name="sequent_backend_area_contest"
                edit={EditAreaContest}
                list={ListAreaContest}
                create={CreateAreaContest}
                options={{label: "Area Contest"}}
            />
            <Resource
                name="sequent_backend_tenant"
                edit={EditTenant}
                list={ListTenant}
                create={CreateTenant}
                options={{label: "Customer"}}
            />
            <Resource
                name="sequent_backend_document"
                show={ShowDocument}
                list={ListDocument}
                create={CreateDocument}
                options={{label: "Document"}}
            />
            <Resource
                name="sequent_backend_trustee"
                edit={EditTrustee}
                list={ListTrustee}
                create={CreateTrustee}
                options={{label: "Trustee"}}
            />
        </Admin>
    )
}

export default App
