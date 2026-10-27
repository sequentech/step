// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {renderToStaticMarkup} from "react-dom/server"
import {ElectionEventListContent} from "./ElectionEventList"
import {IPermissions} from "@/types/keycloak"

const mockPermissions = new Set<IPermissions>()
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        isAuthorized: (_all: boolean, _tenant: string, permission: IPermissions) =>
            mockPermissions.has(permission),
    }),
}))
jest.mock("react-admin", () => ({
    Button: ({label}: {label: string}) => <button>{label}</button>,
    useGetList: () => ({data: [], isLoading: false, refetch: jest.fn()}),
}))
jest.mock("react-router-dom", () => ({
    useLocation: () => ({pathname: "/"}),
    useNavigate: () => jest.fn(),
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (value: string) => value})}))
jest.mock("jotai", () => ({atom: jest.fn(), useAtom: () => [false]}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/providers/CreateElectionEventContextProvider", () => ({
    useCreateElectionEventStore: () => ({openCreateDrawer: jest.fn(), openImportDrawer: jest.fn()}),
}))
jest.mock("@/components/election-event/create/CreateElectionEventDrawer", () => ({
    CreateDataDrawer: () => null,
}))
jest.mock("@/components/election-event/import-data/ImportDataDrawer", () => ({
    ImportDataDrawer: () => null,
}))

it("offers the empty Preparation context to a board member holding create but no election-data write", () => {
    mockPermissions.clear()
    mockPermissions.add(IPermissions.SIGNING_REQUESTS_CANCEL)
    mockPermissions.add(IPermissions.ELECTION_EVENT_CREATE)
    expect(renderToStaticMarkup(<ElectionEventListContent />)).toContain("common.label.add")
})
it("does not offer event creation to an account lacking create permission", () => {
    mockPermissions.clear()
    mockPermissions.add(IPermissions.ELECTION_EVENT_WRITE)
    expect(renderToStaticMarkup(<ElectionEventListContent />)).not.toContain("common.label.add")
})
