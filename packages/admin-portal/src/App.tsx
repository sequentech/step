// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Admin, CustomRoutes, DataProvider, Resource, useGetOne} from "react-admin"
import React, {useContext, useEffect, useMemo, useState} from "react"
import {ElectionEventBaseTabs} from "./resources/ElectionEvent/ElectionEventBaseTabs"

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
import {Messages} from "./screens/Messages"
import {Route} from "react-router-dom"
import {ShowDocument} from "./resources/Document/ShowDocument"
import {UserAndRoles} from "./screens/UserAndRoles"
import buildHasuraProvider from "ra-data-hasura"
import {customBuildQuery} from "./queries/customBuildQuery"
import {fullAdminTheme} from "./services/AdminTheme"
import {SettingsScreen} from "./screens/SettingsScreen"
import {ListUsers} from "./resources/User/ListUsers"
import {CreateElectionList} from "./resources/ElectionEvent/CreateElectionEvent"
import {CustomLayout} from "./components/CustomLayout"
import {EditBallotStyle} from "./resources/BallotStyle/EditBallotStyle"
import {EditArea} from "./resources/Area/EditArea"
import {EditAreaContest} from "./resources/AreaContest/EditAreaContest"
import {EditTenant} from "./resources/Tenant/EditTenant"
import {CreateTenant} from "./resources/Tenant/CreateTenant"
import {CreateElection} from "./resources/Election/CreateElection"
import {ElectionBaseTabs} from "./resources/ElectionEvent/ElectionBaseTabs"
import {CandidateBaseTabs} from "./resources/Candidate/CandidateBaseTabs"
import {ContestBaseTabs} from "./resources/Contest/ContestBaseTabs"
import {SettingsElectionsTypesCreate} from "./resources/Settings/SettingsElectionsTypesCreate"
import {adminI18nProvider} from "./services/AdminTranslation"
import {useTranslation} from "react-i18next"
import {CommunicationTemplateEdit} from "./resources/CommunicationTemplate/CommunicationTemplateEdit"
import {CommunicationTemplateList} from "./resources/CommunicationTemplate/CommunicationTemplateList"
import {CommunicationTemplateCreate} from "./resources/CommunicationTemplate/CommunicationTemplateCreate"
import {ApolloContext} from "./providers/ApolloContextProvider"
import {TenantContext} from "./providers/TenantContextProvider"
import {Sequent_Backend_Tenant} from "./gql/graphql"
import {LoginScreen} from "./screens/LoginScreen"

interface AppProps {}

const App: React.FC<AppProps> = () => {
    const {apolloClient} = useContext(ApolloContext)
    const [dataProvider, setDataProvider] = useState<DataProvider | null>(null)
    const {i18n, t} = useTranslation()
    adminI18nProvider.changeLocale(i18n.language)
    i18n.on("languageChanged", (lng) => adminI18nProvider.changeLocale(lng))

    useEffect(() => {
        const buildDataProvider = async () => {
            const options = {
                client: apolloClient as any,
                buildQuery: customBuildQuery as any,
            }
            const buildGqlQueryOverrides = {}
            const dataProviderHasura = await buildHasuraProvider(options, buildGqlQueryOverrides)
            setDataProvider(() => dataProviderHasura as any)
        }
        buildDataProvider()
    }, [])

    if (!dataProvider) return <p>{t("loadingDataProvider")}</p>

    return (
        <Admin
            dataProvider={dataProvider || undefined}
            layout={CustomLayout}
            theme={fullAdminTheme}
            i18nProvider={adminI18nProvider}
            loginPage={LoginScreen}
        >
            <CustomRoutes>
                {/*<Route path="/logs" element={<Logs />} />*/}
                <Route path="/user-roles" element={<UserAndRoles />} />
                <Route path="/messages" element={<Messages />} />
                <Route path="/settings/" element={<SettingsScreen />} />
                <Route path="/login/:tenantId?" element={<LoginScreen />} />
            </CustomRoutes>

            <Resource
                name="sequent_backend_election_event"
                list={ElectionEventList}
                create={CreateElectionList}
                edit={ElectionEventBaseTabs}
                show={ElectionEventBaseTabs}
                options={{label: "Election Events", isMenuParent: true}}
            />

            <Resource
                name="sequent_backend_election_type"
                create={SettingsElectionsTypesCreate}
                edit={SettingsScreen}
                show={SettingsScreen}
                options={{label: "Election Type", isMenuParent: true}}
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
                list={ListContest}
                create={CreateContest}
                edit={ContestBaseTabs}
                show={ContestBaseTabs}
                options={{
                    label: "Contests",
                    menuParent: "sequent_backend_election",
                    foreignKeyFrom: "election_id",
                }}
            />
            <Resource
                name="sequent_backend_candidate"
                list={ListCandidate}
                create={CreateCandidate}
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
                name="sequent_backend_communication_template"
                edit={CommunicationTemplateEdit}
                list={CommunicationTemplateList}
                create={CommunicationTemplateCreate}
                options={{label: "Communication Template"}}
            />
            <Resource name="user" edit={EditArea} list={ListUsers} options={{label: "Users"}} />
        </Admin>
    )
}

export default App
