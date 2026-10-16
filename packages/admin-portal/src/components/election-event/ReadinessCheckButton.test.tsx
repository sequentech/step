/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {ReadinessCheckButton} from "./ReadinessCheckButton"
import {IPermissions} from "@/types/keycloak"

const mockRun = jest.fn()
const mockDownload = jest.fn()
const mockQuery = jest.fn()
const mockRefresh = jest.fn()
let mockAuthorized = true
const official = "22222222-2222-4222-8222-222222222222"
const tested = "33333333-3333-4333-8333-333333333333"
const other = "44444444-4444-4444-8444-444444444444"
const event = (id: string, name: string, archived = false) => ({
    id,
    presentation: {i18n: {en: {name}}},
    is_archived: archived,
})
const mockEvents = [
    event(official, "Official"),
    event(tested, "Rehearsal", true),
    event(other, "Other rehearsal"),
]
const report = (passed = true) => ({
    passed,
    checked_at: "2026-10-16T09:00:00Z",
    checks: [
        "configuration",
        "certified_voters",
        "official_zero",
        "publication",
        "tested_finished",
    ].map((kind) => ({
        kind,
        passed: passed || kind !== "certified_voters",
        message: "Server explanation",
    })),
    evidence: {official: {event_id: official}, tested: {event_id: tested}},
    log_entry_id: passed ? "signed-entry-1" : null,
})

jest.mock("@apollo/client", () => ({
    gql: jest.fn(),
    useMutation: () => [mockRun],
    useQuery: (...args: unknown[]) => mockQuery(...args),
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("react-admin", () => ({useRefresh: () => mockRefresh}))
jest.mock("@sequentech/ui-core", () => ({
    downloadBlob: (...args: unknown[]) => mockDownload(...args),
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant-1"]}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthorized: () => mockAuthorized}),
}))
jest.mock("@/hooks/useAliasRenderer", () => ({
    useAliasRenderer: () => (value: {presentation: {i18n: {en: {name: string}}}}) =>
        value.presentation.i18n.en.name,
}))

beforeEach(() => {
    jest.clearAllMocks()
    mockAuthorized = true
    mockQuery.mockReturnValue({
        data: {sequent_backend_election_event: mockEvents},
        loading: false,
        refetch: jest.fn(),
    })
    mockRun.mockResolvedValue({data: {readiness_check: report()}})
    mockDownload.mockResolvedValue(undefined)
})

function open() {
    render(<ReadinessCheckButton event={event(official, "Official")} />)
    fireEvent.click(screen.getByRole("button", {name: "readiness.action"}))
}
function select(name = "Rehearsal") {
    fireEvent.change(screen.getByRole("combobox", {name: "readiness.testedEvent"}), {
        target: {value: name},
    })
    fireEvent.click(screen.getByRole("option", {name: new RegExp(name)}))
}
async function run() {
    fireEvent.click(screen.getByRole("button", {name: "readiness.run"}))
    await screen.findByText("signed-entry-1")
}

it("does not expose the action or fetch events without readiness permission", () => {
    mockAuthorized = false
    render(<ReadinessCheckButton event={event(official, "Official")} />)
    expect(screen.queryByRole("button")).not.toBeInTheDocument()
    expect(mockQuery).not.toHaveBeenCalled()
})
it("requires a different tested event, including archived rehearsals", () => {
    open()
    expect(screen.getByRole("button", {name: "readiness.run"})).toBeDisabled()
    select()
    expect(screen.getByRole("button", {name: "readiness.run"})).toBeEnabled()
    expect(mockQuery.mock.calls[0][1]).toMatchObject({
        variables: {tenantId: "tenant-1", officialEventId: official},
        context: {headers: {"x-hasura-role": IPermissions.ELECTION_EVENT_READ}},
    })
})
it("runs as readiness-check and displays and downloads the signed report", async () => {
    open()
    select()
    await run()
    expect(mockRun).toHaveBeenCalledWith({
        variables: {official_event_id: official, tested_event_id: tested},
        context: {headers: {"x-hasura-role": IPermissions.READINESS_CHECK}},
    })
    expect(screen.getByText("readiness.success")).toBeVisible()
    expect(screen.getByText(/2026-10-16T09:00:00Z/)).toBeVisible()
    expect(mockRefresh).toHaveBeenCalledTimes(1)
    fireEvent.click(screen.getByRole("button", {name: "readiness.download"}))
    await waitFor(() => expect(mockDownload).toHaveBeenCalledTimes(1))
    const [blob, name] = mockDownload.mock.calls[0]
    expect(blob.type).toBe("application/json")
    expect(name).toBe(`readiness-${official}.json`)
    const contents = await new Promise<string>((resolve) => {
        const reader = new FileReader()
        reader.onload = () => resolve(String(reader.result))
        reader.readAsText(blob)
    })
    expect(JSON.parse(contents)).toEqual(report())
})
it("shows failed checks and retains their evidence for download", async () => {
    mockRun.mockResolvedValue({data: {readiness_check: report(false)}})
    open()
    select()
    fireEvent.click(screen.getByRole("button", {name: "readiness.run"}))
    expect(await screen.findByText("readiness.failed")).toBeVisible()
    expect(screen.getByText("logsScreen.readiness.checks.certified_voters")).toBeVisible()
    expect(screen.getByRole("button", {name: "readiness.download"})).toBeEnabled()
    expect(screen.queryByText("readiness.success")).not.toBeInTheDocument()
    expect(mockRefresh).not.toHaveBeenCalled()
})
it.each([
    ["transport failure", () => Promise.reject(new Error("Network unavailable"))],
    [
        "GraphQL failure with partial data",
        () => Promise.resolve({data: {readiness_check: report()}, errors: [{message: "denied"}]}),
    ],
    ["missing report", () => Promise.resolve({data: {readiness_check: null}})],
    [
        "unsigned pass",
        () => Promise.resolve({data: {readiness_check: {...report(), log_entry_id: null}}}),
    ],
    [
        "inconsistent verdict",
        () =>
            Promise.resolve({
                data: {readiness_check: {...report(false), passed: true, log_entry_id: "entry"}},
            }),
    ],
])("never displays success for %s and allows an explicit retry", async (_name, answer) => {
    mockRun.mockImplementationOnce(answer)
    open()
    select()
    fireEvent.click(screen.getByRole("button", {name: "readiness.run"}))
    expect(await screen.findByText("readiness.error")).toBeVisible()
    expect(screen.queryByText("readiness.success")).not.toBeInTheDocument()
    expect(screen.queryByRole("button", {name: "readiness.download"})).not.toBeInTheDocument()
    await run()
})
it("disables duplicate submissions and changing the comparison while pending", async () => {
    let finish!: (value: unknown) => void
    mockRun.mockImplementationOnce(
        () =>
            new Promise((resolve) => {
                finish = resolve
            })
    )
    open()
    select()
    fireEvent.click(screen.getByRole("button", {name: "readiness.run"}))
    expect(screen.getByRole("combobox")).toBeDisabled()
    expect(screen.getByRole("button", {name: "readiness.running"})).toBeDisabled()
    fireEvent.click(screen.getByRole("button", {name: "readiness.running"}))
    expect(mockRun).toHaveBeenCalledTimes(1)
    await act(async () => {
        finish({data: {readiness_check: report()}})
    })
    expect(await screen.findByText("signed-entry-1")).toBeVisible()
})
it("clears the previous report when a different rehearsal is selected", async () => {
    open()
    select()
    await run()
    select("Other rehearsal")
    expect(screen.queryByText("readiness.success")).not.toBeInTheDocument()
    expect(screen.queryByText("signed-entry-1")).not.toBeInTheDocument()
})
it("does not permit running against an unavailable event list", async () => {
    mockQuery.mockReturnValue({loading: false, error: new Error("denied"), refetch: jest.fn()})
    open()
    expect(screen.getByText("readiness.eventsError")).toBeVisible()
    expect(screen.getByRole("button", {name: "readiness.run"})).toBeDisabled()
})
