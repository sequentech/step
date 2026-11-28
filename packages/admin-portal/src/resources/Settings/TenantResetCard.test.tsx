/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {cleanup, fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {blockingRefusals, TenantResetCard, type TenantResetPreview} from "./TenantResetCard"

const TENANT = "11111111-1111-4111-8111-111111111111"
const mockReset = jest.fn()
const mockLoad = jest.fn()
const mockRefetch = jest.fn()
const mockRefetchTask = jest.fn()
const mockReauthenticate = jest.fn()
let mockAuthorized = true
let mockPreview: TenantResetPreview
type Task = {id: string; execution_status: string; annotations: unknown; logs: unknown}
// null: nothing read yet; []: the task can't be found.
let mockTasks: Task[] | null
let mockPreviewError: Error | undefined

const preview = (over: Partial<TenantResetPreview> = {}): TenantResetPreview => ({
    enabled: true,
    has_baseline: true,
    baseline_captured_at: "2026-11-28 09:00:00+00",
    resetting: false,
    election_events: [{id: "e1", name: "General election"}],
    rows: {election_event: 1, election: 2},
    cast_ballots: 0,
    sealed_ballots: 0,
    refusals: [],
    erase_ballots_phrase: "ERASE BALLOTS",
    slug: "acme",
    fresh_login_required: false,
    ...over,
})

jest.mock("@apollo/client", () => ({
    gql: jest.fn(),
    useLazyQuery: () => [
        mockLoad,
        {
            data: mockPreviewError ? undefined : {tenant_reset_preview: mockPreview},
            error: mockPreviewError,
            loading: false,
            refetch: mockRefetch,
        },
    ],
    useMutation: () => [mockReset],
    useQuery: () => ({
        data: mockTasks ? {sequent_backend_tasks_execution: mockTasks} : undefined,
        loading: false,
        refetch: mockRefetchTask,
    }),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key}),
}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        isAuthorized: () => mockAuthorized,
        reauthenticate: () => mockReauthenticate(),
    }),
}))

beforeEach(() => {
    jest.clearAllMocks()
    mockAuthorized = true
    mockPreview = preview()
    mockTasks = null
    mockPreviewError = undefined
    mockLoad.mockResolvedValue({})
    mockRefetch.mockResolvedValue({})
    mockRefetchTask.mockResolvedValue({})
    mockReset.mockResolvedValue({data: {reset_tenant: {task_execution: {id: "task-1"}}}})
})
afterEach(cleanup)

const openDialog = () => {
    render(<TenantResetCard tenantId={TENANT} />)
    fireEvent.click(screen.getByRole("button", {name: "configurationPackage.reset.open"}))
}
const confirmButton = () => screen.getByTestId("tenant-reset-confirm")
const type = (testId: string, value: string) =>
    fireEvent.change(screen.getByTestId(testId), {target: {value}})

test("is hidden without the reset permission", () => {
    mockAuthorized = false
    render(<TenantResetCard tenantId={TENANT} />)
    expect(screen.queryByTestId("tenant-reset-card")).not.toBeInTheDocument()
})

test("resets only after the tenant's identifier is typed", async () => {
    openDialog()
    expect(confirmButton()).toBeDisabled()
    type("tenant-reset-confirmation", "acm")
    expect(confirmButton()).toBeDisabled()
    type("tenant-reset-confirmation", "acme")
    expect(confirmButton()).toBeEnabled()
    fireEvent.click(confirmButton())
    await waitFor(() =>
        expect(mockReset).toHaveBeenCalledWith({
            variables: {
                tenant_id: TENANT,
                confirmation: "acme",
                erase_ballots_confirmation: null,
            },
        })
    )
    expect(await screen.findByText("configurationPackage.reset.running")).toBeInTheDocument()
})

test("shows what the server refused and keeps the reset off", () => {
    mockPreview = preview({refusals: [{kind: "voting_open", election_event_id: "e1"}]})
    openDialog()
    expect(screen.getByTestId("tenant-reset-refusals")).toHaveTextContent(
        "configurationPackage.reset.refusal.voting_open"
    )
    type("tenant-reset-confirmation", "acme")
    expect(confirmButton()).toBeDisabled()
})

test("asks for a recent sign-in before a reset", () => {
    mockPreview = preview({fresh_login_required: true})
    openDialog()
    type("tenant-reset-confirmation", "acme")
    expect(confirmButton()).toBeDisabled()
    fireEvent.click(screen.getByRole("button", {name: "configurationPackage.reset.signIn"}))
    expect(mockReauthenticate).toHaveBeenCalled()
})

test("erases ballots only where allowed and after the phrase", async () => {
    mockPreview = preview({
        cast_ballots: 3,
        refusals: [{kind: "ballots", cast: 3, sealed: 0, overridable: true}],
    })
    openDialog()
    type("tenant-reset-confirmation", "acme")
    expect(confirmButton()).toBeDisabled()
    type("tenant-reset-phrase", "ERASE BALLOTS")
    expect(confirmButton()).toBeEnabled()
    fireEvent.click(confirmButton())
    await waitFor(() =>
        expect(mockReset).toHaveBeenCalledWith({
            variables: {
                tenant_id: TENANT,
                confirmation: "acme",
                erase_ballots_confirmation: "ERASE BALLOTS",
            },
        })
    )
})

test("offers to finish a reset that did not finish", () => {
    mockPreview = preview({resetting: true})
    render(<TenantResetCard tenantId={TENANT} />)
    expect(screen.getByTestId("tenant-reset-unfinished")).toBeInTheDocument()
    expect(
        screen.getByRole("button", {name: "configurationPackage.reset.finish"})
    ).toBeInTheDocument()
})

test("a ballot refusal the deployment does not allow stays", () => {
    const fixed = {kind: "ballots" as const, cast: 1, sealed: 0, overridable: false}
    const allowed = {...fixed, overridable: true}
    expect(blockingRefusals([fixed], true)).toEqual([fixed])
    expect(blockingRefusals([allowed], true)).toEqual([])
    expect(blockingRefusals([allowed], false)).toEqual([allowed])
})

const startReset = async (onReset?: () => void) => {
    const view = render(<TenantResetCard tenantId={TENANT} onReset={onReset} />)
    fireEvent.click(screen.getByRole("button", {name: "configurationPackage.reset.open"}))
    type("tenant-reset-confirmation", "acme")
    fireEvent.click(confirmButton())
    expect(await screen.findByText("configurationPackage.reset.running")).toBeInTheDocument()
    return view
}

test("tells the page once the reset succeeded", async () => {
    const onReset = jest.fn()
    const {rerender} = await startReset(onReset)
    mockTasks = [{id: "task-1", execution_status: "SUCCESS", annotations: {}, logs: []}]
    rerender(<TenantResetCard tenantId={TENANT} onReset={onReset} />)
    expect(await screen.findByTestId("tenant-reset-succeeded")).toBeInTheDocument()
    expect(onReset).toHaveBeenCalledTimes(1)
})

test("a task that can't be read leaves the outcome unknown and the dialog closable", async () => {
    const {rerender} = await startReset()
    mockTasks = []
    rerender(<TenantResetCard tenantId={TENANT} />)
    expect(await screen.findByTestId("tenant-reset-unknown")).toBeInTheDocument()
    expect(screen.getByRole("button", {name: "configurationPackage.reset.close"})).toBeEnabled()
    fireEvent.click(screen.getByRole("button", {name: "configurationPackage.reset.checkAgain"}))
    expect(mockRefetchTask).toHaveBeenCalled()
    expect(mockReset).toHaveBeenCalledTimes(1)
})

test("a preview that fails to load shows the error and can be retried", () => {
    mockPreviewError = new Error("network down")
    openDialog()
    expect(screen.getByTestId("tenant-reset-preview-error")).toHaveTextContent("network down")
    mockRefetch.mockClear()
    fireEvent.click(screen.getByRole("button", {name: "configurationPackage.reset.retry"}))
    expect(mockRefetch).toHaveBeenCalledWith({tenant_id: TENANT})
})
