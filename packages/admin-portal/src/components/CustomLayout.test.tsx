/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen, fireEvent, cleanup} from "@testing-library/react"
import "@testing-library/jest-dom"
import {CoreAdminContext, testDataProvider} from "react-admin"
import {CustomLayout} from "./CustomLayout"
import {useSigningRequest} from "./signing/SigningProvider"

jest.mock("react-admin", () => {
    const {TextEncoder, TextDecoder} = jest.requireActual("node:util")
    Object.assign(globalThis, {TextEncoder, TextDecoder})
    return {
        ...jest.requireActual("ra-core"),
        Layout: ({children}: {children: React.ReactNode}) => <>{children}</>,
    }
})
jest.mock("./CustomAppBar", () => ({CustomAppBar: () => null}))
jest.mock("./CustomMenu", () => ({CustomMenu: () => null}))
jest.mock("./menu/CustomSidebar", () => ({CustomSidebar: () => null}))
jest.mock("./election-event/import-data/ImportDataDrawer", () => ({ImportDataDrawer: () => null}))
jest.mock("./election-event/create/CreateElectionEventDrawer", () => ({
    CreateDataDrawer: () => null,
}))
jest.mock("@/providers/CreateElectionEventContextProvider", () => ({
    CreateElectionEventProvider: () => null,
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: jest.requireActual("react").createContext({
        isAuthenticated: false,
        userId: "operator",
        isAuthorized: () => true,
    }),
}))
jest.mock("@apollo/client", () => ({useApolloClient: () => mockClient}))
jest.mock("@/lib/signing/api", () => ({createSigningApi: () => ({})}))
jest.mock("@sequentech/ui-core", () => ({}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("@/resources/Publish/ClosedVotingCard", () => ({ClosedVotingCard: () => null}))
jest.mock("@/resources/Reports/ReportSigning", () => ({
    ReportCompletionActions: () => null,
    TransmissionCompletionActions: () => null,
}))
jest.mock("./signing/SigningRequestPanel", () => ({
    SigningRequestPanel: ({requestId}: {requestId: string}) => {
        const {useGetOne} = jest.requireActual("ra-core")
        const {data} = useGetOne("sequent_backend_election", {id: "post"})
        return (
            <div>
                {requestId}: {data?.name}
            </div>
        )
    },
}))
const mockClient = {}
const provider = testDataProvider({getOne: async () => ({data: {id: "post", name: "Madrid Post"}})})
function Launch() {
    const signing = useSigningRequest()
    return <button onClick={() => signing.open("request")}>Open request</button>
}
afterEach(cleanup)
it("hosts the signing overlay inside the admin data and query contexts", async () => {
    const view = render(
        <CoreAdminContext dataProvider={provider}>
            <CustomLayout>
                <Launch />
            </CustomLayout>
        </CoreAdminContext>
    )
    fireEvent.click(screen.getByText("Open request"))
    expect(await screen.findByText("request: Madrid Post")).toBeVisible()
    view.rerender(
        <CoreAdminContext dataProvider={provider}>
            <CustomLayout>
                <Launch />
                <div>Another screen</div>
            </CustomLayout>
        </CoreAdminContext>
    )
    expect(screen.getByText("request: Madrid Post")).toBeVisible()
})
