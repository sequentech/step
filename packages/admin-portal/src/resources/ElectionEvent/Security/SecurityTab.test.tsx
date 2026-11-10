/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor, within} from "@testing-library/react"
import SecurityTab, {
    GET_SECURITY_TESTS,
    RUN_SECURITY_TESTS,
    RUN_TSF_INTEGRITY_CHECK,
    SAVE_SECURITY_SETTINGS,
    START_POST_PROCESSING,
} from "./SecurityTab"
import {IPermissions} from "@/types/keycloak"

let mockPhase = "preparation"
let mockArchived = false
let mockRuns: unknown[] = []
let mockTsfIntegrity: unknown = null
let mockFresh = false
let mockRoles = new Set<string>()
const mockNotify = jest.fn()
const mockReauthenticate = jest.fn(async (_url: string) => {})
let mockBaselinePresent = true
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
                    baseline_present: mockBaselinePresent,
                    notifications: [],
                    runs: mockRuns,
                    tsf_integrity: mockTsfIntegrity,
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
    if (request.mutation === RUN_TSF_INTEGRITY_CHECK) {
        mockTsfIntegrity = {
            id: "pending-check",
            requested_by: "admin1",
            started_at: "2026-10-27T12:00:00Z",
            finished_at: null,
            outcome: null,
            result: {},
        }
        return {data: {run_tsf_integrity_check: {task_id: "pending-check", state: "queued"}}}
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
    useRecordContext: () => ({id: "event", is_archived: mockArchived}),
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
    mockBaselinePresent = true
    mockPhase = "preparation"
    mockArchived = false
    mockFresh = false
    mockRuns = []
    mockTsfIntegrity = null
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

it("offers only the on-request purpose before the first approval", async () => {
    mockBaselinePresent = false
    render(<SecurityTab />)
    await screen.findByText(/The board-approved configuration baseline is not yet available/)
    fireEvent.mouseDown(screen.getByRole("combobox", {name: "Purpose of the test"}))
    const disabled = async (name: string) =>
        (await screen.findByRole("option", {name})).getAttribute("aria-disabled") === "true"
    expect(await disabled("On request")).toBe(false)
    expect(await disabled("Before starting voting")).toBe(true)
    expect(await disabled("Before resuming voting")).toBe(true)
    expect(await disabled("Before counting")).toBe(true)
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

it("does not offer post-processing in preparation, also for an archived event", async () => {
    mockArchived = true
    render(<SecurityTab />)
    await screen.findByText(/Preparation/)
    expect(screen.queryByRole("button", {name: "Start post-processing"})).toBeNull()
})

it("says when post-processing may start, also after an unrecoverable failure", async () => {
    mockPhase = "evaluation"
    render(<SecurityTab />)
    fireEvent.click(await screen.findByRole("button", {name: "Start post-processing"}))
    const dialog = screen.getByRole("dialog").textContent
    expect(dialog).toContain("the seal of one of its ballot boxes failed")
    expect(dialog).toContain(
        'In the safety state "Unrecoverable failure", post-processing starts without a result'
    )
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

describe("TOE software integrity", () => {
    const VERIFY = "Verify the TOE software"
    const ACCEPTED = `sha256:aaaaaaaa${"0".repeat(50)}bbbbbb`
    const LIST_SHA256 = "c".repeat(64)
    const finished = (outcome: string, result: object) => ({
        id: "check",
        requested_by: "board1",
        started_at: "2026-10-27T12:00:00Z",
        finished_at: "2026-10-27T12:00:01Z",
        outcome,
        result: {
            toe_reference: "Sequent Voting Platform",
            digest_list_sha256: LIST_SHA256,
            parts: [],
            ...result,
        },
    })
    const part = (name: string, outcome: string, detail: string | null = null) => ({
        part: name,
        accepted_digest: ACCEPTED,
        running: 2,
        outcome,
        detail,
    })

    it("reads the latest check with the status", () => {
        expect(GET_SECURITY_TESTS.loc?.source.body).toContain("tsf_integrity")
    })

    it("starts a check with the run permission, apart from the suite, and prevents a duplicate click", async () => {
        render(<SecurityTab />)
        const verify = await screen.findByRole("button", {name: VERIFY})
        await waitFor(() => expect((verify as HTMLButtonElement).disabled).toBe(false))
        expect(
            screen.getByText("The TOE software has not been verified for this election event yet.")
        ).toBeTruthy()
        fireEvent.click(verify)
        await waitFor(() =>
            expect(mockMutate).toHaveBeenCalledWith(
                expect.objectContaining({
                    mutation: RUN_TSF_INTEGRITY_CHECK,
                    variables: {event: "event"},
                    context: {headers: {"x-hasura-role": IPermissions.SELF_TEST_RUN}},
                })
            )
        )
        await waitFor(() =>
            expect(
                (
                    screen.getByRole("button", {
                        name: "Verification queued or running",
                    }) as HTMLButtonElement
                ).disabled
            ).toBe(true)
        )
        expect(screen.getByText("admin1")).toBeTruthy()
        expect(
            mockMutate.mock.calls.some(([request]) => request.mutation === RUN_SECURITY_TESTS)
        ).toBe(false)
        expect(
            (screen.getByRole("button", {name: "Run self-tests"}) as HTMLButtonElement).disabled
        ).toBe(false)
    })

    it("does not let a user without the run permission start a check", async () => {
        mockRoles = new Set([IPermissions.SELF_TEST_READ])
        mockTsfIntegrity = finished("verified", {})
        render(<SecurityTab />)
        await screen.findByText("Matches the accepted delivery")
        expect((screen.getByRole("button", {name: VERIFY}) as HTMLButtonElement).disabled).toBe(
            true
        )
    })

    it("shows a match with who ran it, the TOE reference, the parts and the digest list", async () => {
        mockTsfIntegrity = finished("verified", {parts: [part("braid", "verified")]})
        render(<SecurityTab />)
        await screen.findByText("Matches the accepted delivery")
        expect(screen.getByText("2026-10-27T12:00:00Z")).toBeTruthy()
        expect(screen.getByText("board1")).toBeTruthy()
        expect(screen.getByText("Sequent Voting Platform")).toBeTruthy()
        expect(screen.getByText(LIST_SHA256)).toBeTruthy()
        const row = within(screen.getByRole("table", {name: "TOE parts"})).getByRole("row", {
            name: /^braid/,
        })
        expect(within(row).getByText("aaaaaaaa…bbbbbb")).toHaveProperty("title", ACCEPTED)
        expect(within(row).getByRole("cell", {name: "2"})).toBeTruthy()
        expect(within(row).getByText("Matches")).toBeTruthy()
    })

    it("lists every TOE part of the servers by its name", async () => {
        const names = [
            "admin-portal",
            "b4",
            "ballot-verifier",
            "braid",
            "graphql-engine",
            "harvest",
            "keycloak",
            "tls-endpoint",
            "voting-portal",
            "windmill",
        ]
        mockTsfIntegrity = finished("verified", {
            parts: names.map((name) => part(name, "verified")),
        })
        render(<SecurityTab />)
        await screen.findByText("Matches the accepted delivery")
        const table = within(screen.getByRole("table", {name: "TOE parts"}))
        expect(
            table
                .getAllByRole("row")
                .slice(1)
                .map((row) => within(row).getAllByRole("cell")[0].textContent)
        ).toEqual(names)
    })

    it("names the part that differs and the reason it gives", async () => {
        const detail = "A container uses an image other than the accepted one"
        mockTsfIntegrity = finished("different", {
            parts: [part("harvest", "different", detail), part("braid", "unverifiable", null)],
        })
        render(<SecurityTab />)
        await screen.findByText("Differs from the accepted delivery")
        const row = screen.getByRole("row", {name: /^harvest/})
        expect(within(row).getByText("Differs")).toBeTruthy()
        expect(within(row).getByText(detail)).toBeTruthy()
    })

    it("shows why a check could not verify, without a table when no part was compared", async () => {
        mockTsfIntegrity = finished("unverifiable", {
            reason: "The host image observation is stale",
        })
        render(<SecurityTab />)
        await screen.findByText("Could not verify")
        expect(screen.getByText("The host image observation is stale")).toBeTruthy()
        expect(screen.queryByRole("table", {name: "TOE parts"})).toBeNull()
        expect(screen.getByText(LIST_SHA256)).toBeTruthy()
    })

    it("omits what a check could not read", async () => {
        mockTsfIntegrity = {
            ...finished("unverifiable", {
                reason: "The check did not complete",
                toe_reference: null,
                digest_list_sha256: null,
            }),
        }
        render(<SecurityTab />)
        await screen.findByText("The check did not complete")
        expect(screen.queryByText("TOE reference")).toBeNull()
        expect(screen.queryByText("SHA-256 of the accepted digest list")).toBeNull()
    })
})
