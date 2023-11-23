// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Admin, CustomRoutes, DataProvider, Resource} from "react-admin"
import {ApolloClient, ApolloProvider, NormalizedCacheObject} from "@apollo/client"
import React, {useContext, useEffect, useMemo, useState} from "react"
import {ElectionEventBaseTabs} from "./resources/ElectionEvent/ElectionEventBaseTabs"

import {AuthContext} from "./providers/AuthContextProvider"
import {CreateArea} from "./resources/Area/CreateArea"
import {CreateAreaContest} from "./resources/AreaContest/CreateAreaContest"
import {CreateBallotStyle} from "./resources/BallotStyle/CreateBallotStyle"
import {CreateCandidate} from "./resources/Candidate/CreateCandidate"
import {CreateContest} from "./resources/Contest/CreateContest"
import {CreateDocument} from "./resources/Document/CreateDocument"
import {ElectionEventList} from "./resources/ElectionEvent/ElectionEventList"
import {ListArea} from "./resources/Area/ListArea"
import {ListAreaContest} from "./resources/AreaContest/ListAreaContest"
import {ListBallotStyle} from "./resources/BallotStyle/ListBallotStyle"
import {ListCandidate} from "./resources/Candidate/ListCandidate"
import {ListContest} from "./resources/Contest/ListContest"
import {ListDocument} from "./resources/Document/ListDocument"
import {ListElection} from "./resources/Election/ListElection"
import {ListTenant} from "./resources/Tenant/ListTenant"
import {ListTrustee} from "./resources/Trustee/ListTrustee"
import {Messages} from "./screens/Messages"
import {PgAuditList} from "./resources/PgAudit/PgAuditList"
import {Route} from "react-router-dom"
import {Settings} from "./screens/Settings"
import {ShowDocument} from "./resources/Document/ShowDocument"
import {UserAndRoles} from "./screens/UserAndRoles"
import buildHasuraProvider from "ra-data-hasura"
import {createApolloClient} from "./services/ApolloService"
import {customBuildQuery} from "./queries/customBuildQuery"
import {fullAdminTheme} from "./services/AdminTheme"
import {isNull} from "@sequentech/ui-essentials"
import {ListUsers} from "./resources/User/ListUsers"
import {CreateElectionList} from "./resources/ElectionEvent/CreateElectionEvent"
import {CustomLayout} from "./components/CustomLayout"
import {EditContest} from "./resources/Contest/EditContest"
import {EditCandidate} from "./resources/Candidate/EditCandidate"
import {EditBallotStyle} from "./resources/BallotStyle/EditBallotStyle"
import {EditArea} from "./resources/Area/EditArea"
import {EditAreaContest} from "./resources/AreaContest/EditAreaContest"
import {EditTenant} from "./resources/Tenant/EditTenant"
import {CreateTenant} from "./resources/Tenant/CreateTenant"
import {EditTrustee} from "./resources/Trustee/EditTrustee"
import {CreateTrustee} from "./resources/Trustee/CreateTrustee"
import {CreateElection} from "./resources/Election/CreateElection"
import {ElectionBaseTabs} from "./resources/ElectionEvent/ElectionBaseTabs"
import {CandidateBaseTabs} from "./resources/Candidate/CandidateBaseTabs"
import {CreateCandidateData} from "./resources/Candidate/CreateCandidateData"

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
        if (authContext.isAuthenticated && accessToken) {
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
            {
                // <Resource name="pgaudit" list={PgAuditList} options={{label: "PGAudit"}} />
            }
            <Resource name="user" list={ListUsers} options={{label: "Users"}} />
            <Resource
                name="sequent_backend_election_event"
                list={ElectionEventList}
                create={CreateElectionList}
                edit={ElectionEventBaseTabs}
                show={ElectionEventBaseTabs}
                options={{label: "Election Events", isMenuParent: true}}
            />
            <Resource
                name="sequent_backend_election"
                list={ListElection}
                create={CreateElection}
                show={ElectionBaseTabs}
                edit={ElectionBaseTabs}
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
                list={ListCandidate}
                create={CreateCandidateData}
                edit={CandidateBaseTabs}
                show={CandidateBaseTabs}
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
