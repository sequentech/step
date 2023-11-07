// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {Admin, DataProvider, Resource, CustomRoutes} from "react-admin"
import buildHasuraProvider from "ra-data-hasura"
import {customBuildQuery} from "./queries/customBuildQuery"
import {apolloClient} from "./services/ApolloService"
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
import {adminTheme} from "@sequentech/ui-essentials"

const App = () => {
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
        <Admin dataProvider={dataProvider || undefined} layout={CustomLayout} theme={adminTheme}>
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
                options={{label: "Election Events"}}
            />
            <Resource
                name="sequent_backend_election"
                edit={EditElection}
                list={ListElection}
                create={CreateElection}
                options={{label: "Elections"}}
            />
            <Resource
                name="sequent_backend_contest"
                edit={EditContest}
                list={ListContest}
                create={CreateContest}
                options={{label: "Contests"}}
            />
            <Resource
                name="sequent_backend_candidate"
                edit={EditCandidate}
                list={ListCandidate}
                create={CreateCandidate}
                options={{label: "Candidates"}}
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
