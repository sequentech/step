/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import SecurityTab, {
    GET_SECURITY_TESTS,
    RUN_SECURITY_TESTS,
    SAVE_SECURITY_SETTINGS,
    START_POST_PROCESSING,
} from "./SecurityTab"
import {IPermissions} from "@/types/keycloak"

let mockPhase = "preparation"
let mockRuns: unknown[] = []
let mockFresh = false
let mockRoles = new Set<string>()
const mockNotify = jest.fn()
const mockReauthenticate = jest.fn(async (_url: string) => {})
const mockMutate = jest.fn(async (request) => {
    if (request.mutation === GET_SECURITY_TESTS)
        return {
            data: {
                get_self_tests: {
                    settings: {
                        interval_minutes: 15,
                        timeout_seconds: 60,
                        additional_conditions: [],
                    },
                    baseline_present: true,
                    notifications: [],
                    runs: mockRuns,
                },
            },
        }
    if (request.mutation === RUN_SECURITY_TESTS) {
        mockRuns = [
            {
                run_id: "pending-run",
                generation: 1,
                trigger: request.variables.trigger,
                started_at: "2026-10-27T12:00:00Z",
                finished_at: null,
                outcome: "recoverable-failure",
                checks: [],
            },
        ]
        return {data: {run_self_tests: {task_id: "pending-run", state: "queued"}}}
    }
    return {data: {}}
})
const mockClient = {
    mutate: mockMutate,
    query: jest.fn(async () => ({
        data: {
            get_election_lifecycle: {
                phase: mockPhase,
                safety: "recoverable",
                generation: 1,
                reason: "Test required",
            },
        },
    })),
}
jest.mock("@apollo/client", () => ({
    ...jest.requireActual("@apollo/client"),
    useApolloClient: () => mockClient,
}))
jest.mock("react-admin", () => ({
    useRecordContext: () => ({id: "event"}),
    useNotify: () => mockNotify,
}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        tenantId: "tenant",
        isAuthorized: (_: unknown, _tenant: unknown, role: string) => mockRoles.has(role),
        isFreshBoardAuthentication: () => mockFresh,
        reauthenticateBoard: (url: string) => mockReauthenticate(url),
    }),
}))
jest.mock("@/hooks/useZonedFormat", () => ({
    useEventZonedFormat: () => ({format: (value: string) => value}),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string) =>
            key
                .split(".")
                .reduce(
                    (value: any, part: string) => value?.[part],
                    require("@/translations/en").default.translations
                ) ?? key,
    }),
}))

beforeEach(() => {
    mockPhase = "preparation"
    mockFresh = false
    mockRuns = []
    mockRoles = new Set([
        IPermissions.SELF_TEST_READ,
        IPermissions.SELF_TEST_RUN,
        IPermissions.SELF_TEST_SETTINGS_WRITE,
        IPermissions.ELECTION_EVENT_READ,
        IPermissions.ELECTION_STATE_WRITE,
    ])
    mockMutate.mockClear()
    mockNotify.mockClear()
    mockReauthenticate.mockClear()
})

it("queues a purpose-bound background test with the run permission and prevents a duplicate click", async () => {
    render(<SecurityTab />)
    const run = await screen.findByRole("button", {name: "Run self-tests"})
    await waitFor(() => expect((run as HTMLButtonElement).disabled).toBe(false))
    fireEvent.mouseDown(screen.getByRole("combobox", {name: "Purpose of the test"}))
    fireEvent.click(await screen.findByRole("option", {name: "Before resuming voting"}))
    fireEvent.click(run)
    await waitFor(() =>
        expect(mockMutate).toHaveBeenCalledWith(
            expect.objectContaining({
                mutation: RUN_SECURITY_TESTS,
                variables: {event: "event", trigger: "before-resume"},
                context: {headers: {"x-hasura-role": IPermissions.SELF_TEST_RUN}},
            })
        )
    )
    await waitFor(() =>
        expect(
            (screen.getByRole("button", {name: "Tests queued or running"}) as HTMLButtonElement)
                .disabled
        ).toBe(true)
    )
    expect(screen.getByText("Ballot-box integrity")).toBeTruthy()
    expect(screen.getByText("Clock synchronization")).toBeTruthy()
})

it("keeps settings immutable during execution even for an administrator", async () => {
    mockPhase = "execution"
    render(<SecurityTab />)
    const interval = await screen.findByRole("spinbutton", {name: "Interval in minutes (1–60)"})
    expect((interval as HTMLInputElement).disabled).toBe(true)
    expect(
        (screen.getByRole("button", {name: "Save self-test settings"}) as HTMLButtonElement)
            .disabled
    ).toBe(true)
    expect(
        mockMutate.mock.calls.some(([request]) => request.mutation === SAVE_SECURITY_SETTINGS)
    ).toBe(false)
})

it("refuses an invalid schedule before sending a settings request", async () => {
    render(<SecurityTab />)
    const interval = await screen.findByRole("spinbutton", {name: "Interval in minutes (1–60)"})
    fireEvent.change(interval, {target: {value: "0"}})
    expect(
        (screen.getByRole("button", {name: "Save self-test settings"}) as HTMLButtonElement)
            .disabled
    ).toBe(true)
})

it("requires fresh external authentication before the irreversible phase change", async () => {
    mockPhase = "evaluation"
    render(<SecurityTab />)
    fireEvent.click(await screen.findByRole("button", {name: "Start post-processing"}))
    fireEvent.click(screen.getByRole("button", {name: "Confirm phase change"}))
    await waitFor(() =>
        expect(mockReauthenticate).toHaveBeenCalledWith(expect.stringContaining("tabId=security"))
    )
    expect(
        mockMutate.mock.calls.some(([request]) => request.mutation === START_POST_PROCESSING)
    ).toBe(false)
})

it("lets an administrator read lifecycle and configure tests without any board data permission", async () => {
    mockRoles = new Set([
        IPermissions.ADMIN_USER,
        IPermissions.SELF_TEST_READ,
        IPermissions.SELF_TEST_RUN,
        IPermissions.SELF_TEST_SETTINGS_WRITE,
    ])
    mockClient.query.mockClear()
    render(<SecurityTab />)
    const interval = await screen.findByRole("spinbutton", {name: "Interval in minutes (1–60)"})
    await waitFor(() => expect((interval as HTMLInputElement).disabled).toBe(false))
    expect(mockClient.query).toHaveBeenCalledWith(
        expect.objectContaining({
            context: {headers: {"x-hasura-role": IPermissions.SELF_TEST_READ}},
        })
    )
    expect(screen.queryByRole("button", {name: "Start post-processing"})).toBeNull()
})
