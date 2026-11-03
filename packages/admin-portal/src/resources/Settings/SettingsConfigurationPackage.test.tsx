/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, fireEvent, render, screen} from "@testing-library/react"
import {record} from "./configurationPackage.fixture"

const mockApply = jest.fn()
const mockTaskQuery = jest.fn()
let mockTask: any = null
let mockAllowed = true
let mockTenant = "t1"
let mockTaskError: unknown = undefined
let mockSha = ""
const mockStopPolling = jest.fn()
const mockRefetch = jest.fn(() => Promise.resolve())
const mockHistoryOptions = jest.fn()

jest.mock("@/queries/ApplyConfigurationPackage", () => ({
    APPLY_CONFIGURATION_PACKAGE: "APPLY",
    LIST_PACKAGE_APPLICATIONS: "HISTORY",
    GET_PACKAGE_TASK: "TASK",
}))
jest.mock("@apollo/client", () => ({
    useMutation: () => [mockApply],
    useQuery: (query: string, options: any) => {
        if (query === "TASK") {
            mockTaskQuery(options)
            return {
                data:
                    options.skip || !mockTask
                        ? undefined
                        : {sequent_backend_tasks_execution: mockTask === "none" ? [] : [mockTask]},
                error: options.skip ? undefined : mockTaskError,
                loading: false,
                startPolling: jest.fn(),
                stopPolling: mockStopPolling,
            }
        }
        mockHistoryOptions(options)
        return {data: {sequent_backend_tasks_execution: []}, refetch: mockRefetch}
    },
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
    }),
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => [mockTenant]}))
jest.mock("@/providers/AuthContextProvider", () => {
    const React = require("react")
    return {AuthContext: React.createContext({isAuthorized: () => mockAllowed})}
})
// The real drawer and screen are exercised in SettingsConfigurationPackage.upload.test.tsx.
jest.mock("@/components/election-event/import-data/ImportScreen", () => ({
    ImportScreen: ({doImport}: any) => (
        <button onClick={() => doImport("doc-1", mockSha)} data-testid="drawer-upload">
            upload
        </button>
    ),
}))
jest.mock("@/components/ElectionHeader", () => () => null)

import {SettingsConfigurationPackage} from "./SettingsConfigurationPackage"

const task = (id: string, status: string, annotations: unknown) => ({
    id,
    execution_status: status,
    annotations,
})
const accepted = (id: string) => ({data: {apply_configuration_package: {task_execution: {id}}}})
const applyButton = () => screen.getByTestId("package-apply") as HTMLButtonElement

beforeEach(() => {
    jest.clearAllMocks()
    sessionStorage.clear()
    mockTask = null
    mockAllowed = true
    mockTenant = "t1"
    mockTaskError = undefined
    mockSha = ""
})

/** Click Apply and confirm it: the application is never one click. */
const apply = async () => {
    fireEvent.click(applyButton())
    await act(async () => {
        fireEvent.click(screen.getByTestId("package-apply-confirm"))
    })
}

const upload = async () => {
    fireEvent.click(screen.getByText("configurationPackage.upload"))
    await act(async () => {
        fireEvent.click(screen.getByTestId("drawer-upload"))
    })
}

/** Render, upload and finish a clean preview, ready to apply. */
const previewed = async () => {
    mockApply.mockResolvedValueOnce(accepted("task-p"))
    const view = render(<SettingsConfigurationPackage />)
    await upload()
    mockTask = task("task-p", "SUCCESS", record("previewed"))
    view.rerender(<SettingsConfigurationPackage />)
    return view
}

it("is not offered without every permission", () => {
    mockAllowed = false
    render(<SettingsConfigurationPackage />)
    expect(screen.getByTestId("package-not-permitted")).toBeTruthy()
    expect(screen.queryByText("configurationPackage.upload")).toBeNull()
})

it("previews an upload first, showing members, blockers and the task, and refuses to apply", async () => {
    mockApply.mockResolvedValue(accepted("task-p"))
    const {rerender} = render(<SettingsConfigurationPackage />)
    expect(screen.getByTestId("package-tenant").textContent).toContain("t1")
    await upload()
    expect(mockApply).toHaveBeenCalledTimes(1)
    expect(mockApply.mock.calls[0][0].variables).toEqual({
        tenantId: "t1",
        documentId: "doc-1",
        mode: "preview",
        requirements: {
            sections: [
                "tenant-configuration",
                "tenant-settings",
                "role-permissions",
                "admin-accounts",
                "trustees",
                "election-event",
            ],
            census: "required",
        },
    })
    expect(applyButton().disabled).toBe(true)

    mockTask = task("task-p", "SUCCESS", record("previewed", ["unknown role x"]))
    rerender(<SettingsConfigurationPackage />)
    expect(screen.getByTestId("package-task").textContent).toContain("task-p")
    expect(screen.getByTestId("package-members").textContent).toContain("export_voters-e1.csv")
    expect(screen.getByTestId("section-tenant-configuration").textContent).toContain(
        "unknown role x"
    )
    expect(applyButton().disabled).toBe(true)
    expect(mockStopPolling).toHaveBeenCalled()
})

it("looks the task up within the tenant only", async () => {
    mockApply.mockResolvedValue(accepted("task-p"))
    render(<SettingsConfigurationPackage />)
    await upload()
    const options = mockTaskQuery.mock.calls.at(-1)[0]
    expect(options.variables).toEqual({taskId: "task-p", tenantId: "t1"})
    expect(options.skip).toBe(false)
})

it("applies exactly what was previewed and shows each section's task and reconciliation", async () => {
    const {rerender} = await previewed()
    expect(applyButton().disabled).toBe(false)

    mockApply.mockResolvedValueOnce(accepted("task-a"))
    await apply()
    mockTask = task("task-a", "IN_PROGRESS", null)
    expect(mockApply.mock.calls[1][0].variables).toMatchObject({documentId: "doc-1", mode: "apply"})
    expect(mockApply.mock.calls[1][0].variables.requirements).toEqual(
        mockApply.mock.calls[0][0].variables.requirements
    )
    rerender(<SettingsConfigurationPackage />)
    expect(screen.getByTestId("package-outcome").textContent).toBe("configurationPackage.running")

    mockTask = task("task-a", "SUCCESS", record("applied"))
    rerender(<SettingsConfigurationPackage />)
    expect(screen.getByTestId("package-outcome").textContent).toBe(
        "configurationPackage.outcome.applied"
    )
    expect(screen.getByTestId("section-tenant-configuration").textContent).toContain("task-tc")
    expect(screen.getByTestId("section-tenant-configuration").textContent).toContain("reconciled")
    expect(screen.getByTestId("section-election-event").textContent).toContain("task-ev")
})

it("shows a partial failure, and a refusal with what earlier runs applied", async () => {
    mockApply.mockResolvedValue(accepted("task-x"))
    const {rerender} = render(<SettingsConfigurationPackage />)
    await upload()
    mockTask = task("task-x", "FAILED", record("failed"))
    rerender(<SettingsConfigurationPackage />)
    expect(screen.getByTestId("package-outcome").textContent).toBe(
        "configurationPackage.outcome.failed"
    )
    expect(screen.getByTestId("section-tenant-configuration").textContent).toContain("applied")
    expect(screen.getByTestId("section-election-event").textContent).toContain("import failed")
    expect(screen.getByTestId("section-election-event").textContent).toContain("task-ev")

    // The task row of a refused application is FAILED, as windmill writes it.
    mockTask = task("task-x", "FAILED", record("refused"))
    rerender(<SettingsConfigurationPackage />)
    expect(screen.getByTestId("package-outcome").textContent).toBe(
        "configurationPackage.outcome.refused"
    )
    expect(screen.getByTestId("section-tenant-configuration").textContent).toContain(
        "configurationPackage.earlierApplied"
    )
    expect(screen.getByTestId("section-tenant-configuration").textContent).toContain(
        'configurationPackage.earlierTasks {"tasks":"task-earlier"}'
    )
    expect(applyButton().disabled).toBe(true)
})

it("never resubmits on its own after a failed submission or an unknown outcome", async () => {
    mockApply.mockRejectedValueOnce(new Error("network down"))
    const {rerender} = render(<SettingsConfigurationPackage />)
    await upload()
    expect(screen.getByTestId("package-error").textContent).toContain("network down")
    // The reply was lost, not the request: a task may exist.
    expect(screen.getByTestId("package-error").textContent).toContain(
        "configurationPackage.mayHaveTask"
    )
    rerender(<SettingsConfigurationPackage />)
    expect(mockApply).toHaveBeenCalledTimes(1)

    mockApply.mockResolvedValueOnce({data: null, errors: [{message: "missing permission"}]})
    await upload()
    expect(screen.getByTestId("package-error").textContent).toBe("missing permission")
    expect(mockApply).toHaveBeenCalledTimes(2)

    mockApply.mockResolvedValueOnce(accepted("task-u"))
    await upload()
    mockTask = task("task-u", "FAILED", record("unknown"))
    rerender(<SettingsConfigurationPackage />)
    rerender(<SettingsConfigurationPackage />)
    expect(screen.getByTestId("package-outcome").textContent).toBe(
        "configurationPackage.outcome.unknown"
    )
    expect(mockApply).toHaveBeenCalledTimes(3)
})

it("resumes the task it was following after a reload in the same tenant", async () => {
    mockApply.mockResolvedValue(accepted("task-p"))
    const first = render(<SettingsConfigurationPackage />)
    await upload()
    first.unmount()
    mockTaskQuery.mockClear()
    mockTask = task("task-p", "SUCCESS", record("previewed"))
    render(<SettingsConfigurationPackage />)
    expect(mockTaskQuery.mock.calls[0][0].variables).toEqual({taskId: "task-p", tenantId: "t1"})
    expect(screen.getByTestId("package-task").textContent).toContain("task-p")
    // The resumed preview may be applied, for the same document and requirements.
    expect(applyButton().disabled).toBe(false)
})

it("clears the preview approval when the requirements change", async () => {
    await previewed()
    expect(applyButton().disabled).toBe(false)
    fireEvent.click(screen.getByLabelText("census"))
    expect(applyButton().disabled).toBe(true)
    // Changing them back still needs a new preview.
    fireEvent.click(screen.getByLabelText("census"))
    expect(applyButton().disabled).toBe(true)
})

it("clears the preview approval when the document changes", async () => {
    await previewed()
    expect(applyButton().disabled).toBe(false)
    fireEvent.change(screen.getByLabelText("configurationPackage.documentId"), {
        target: {value: "doc-2"},
    })
    expect(applyButton().disabled).toBe(true)
})

it("submits nothing once a permission is lost", async () => {
    const {rerender} = await previewed()
    mockAllowed = false
    rerender(<SettingsConfigurationPackage />)
    expect(screen.queryByTestId("package-apply")).toBeNull()
    expect(mockApply).toHaveBeenCalledTimes(1)
})

it("can preview a document uploaded before, as the CLI's --document-id does", async () => {
    mockApply.mockResolvedValue(accepted("task-p"))
    render(<SettingsConfigurationPackage />)
    fireEvent.change(screen.getByLabelText("configurationPackage.documentId"), {
        target: {value: " doc-9 "},
    })
    await act(async () => {
        fireEvent.click(screen.getByText("configurationPackage.preview"))
    })
    expect(mockApply.mock.calls[0][0].variables).toMatchObject({
        documentId: "doc-9",
        mode: "preview",
    })
})

it("asks for confirmation before applying, and applies nothing when cancelled", async () => {
    await previewed()
    fireEvent.click(applyButton())
    fireEvent.click(screen.getByText("configurationPackage.cancel"))
    expect(mockApply).toHaveBeenCalledTimes(1)
})

describe("after an application ends, Apply needs a fresh preview", () => {
    it.each([
        ["failed", "FAILED"],
        ["unknown", "FAILED"],
        ["refused", "FAILED"],
        ["applied", "SUCCESS"],
    ])("after a %s application", async (outcome, status) => {
        const {rerender} = await previewed()
        mockApply.mockResolvedValueOnce(accepted("task-a"))
        await apply()
        mockTask = task("task-a", status, record(outcome))
        rerender(<SettingsConfigurationPackage />)
        rerender(<SettingsConfigurationPackage />)
        expect(applyButton().disabled).toBe(true)
    })

    it("after an apply mutation that was rejected", async () => {
        await previewed()
        mockApply.mockRejectedValueOnce(new Error("timeout"))
        await apply()
        expect(applyButton().disabled).toBe(true)
        expect(screen.getByTestId("package-error").textContent).toContain(
            "configurationPackage.mayHaveTask"
        )
    })
})

it("says plainly when a task stopped without a record, with its last log line", async () => {
    mockApply.mockResolvedValue(accepted("task-f"))
    const {rerender} = render(<SettingsConfigurationPackage />)
    await upload()
    // What update_fail writes: annotations replaced, the reason only in the log.
    mockTask = {
        ...task("task-f", "FAILED", {document_id: null}),
        logs: [
            {created_date: "2026-10-25T00:00:00Z", log_text: "Task started"},
            {
                created_date: "2026-10-25T00:00:01Z",
                log_text:
                    "Error: another configuration package application on this tenant is running",
            },
        ],
    }
    rerender(<SettingsConfigurationPackage />)
    expect(screen.getByTestId("package-outcome").textContent).toBe(
        "configurationPackage.stoppedWithoutRecord"
    )
    expect(screen.getByTestId("package-last-log").textContent).toContain(
        "another configuration package application on this tenant is running"
    )
    expect(screen.queryByText("configurationPackage.outcome.unknown")).toBeNull()
    expect(applyButton().disabled).toBe(true)
})

it("treats a CANCELLED task as finished, not running", async () => {
    mockApply.mockResolvedValue(accepted("task-c"))
    const {rerender} = render(<SettingsConfigurationPackage />)
    await upload()
    mockTask = task("task-c", "CANCELLED", {document_id: null})
    rerender(<SettingsConfigurationPackage />)
    expect(screen.getByTestId("package-outcome").textContent).toBe(
        "configurationPackage.stoppedWithoutRecord"
    )
    expect(mockStopPolling).toHaveBeenCalled()
    expect((screen.getByText("configurationPackage.upload") as HTMLButtonElement).disabled).toBe(
        false
    )
})

describe("a task that cannot be read", () => {
    it.each([
        ["a query error", () => (mockTaskError = new Error("permission denied"))],
        ["no row in this tenant", () => (mockTask = "none")],
    ])("is reported on %s, and can be stopped following", async (_, make) => {
        mockApply.mockResolvedValue(accepted("task-r"))
        const {rerender} = render(<SettingsConfigurationPackage />)
        await upload()
        make()
        rerender(<SettingsConfigurationPackage />)
        expect(screen.getByTestId("package-outcome").textContent).toContain(
            "configurationPackage.unreadable"
        )
        // Not locked: the operator can upload again or stop following.
        expect(
            (screen.getByText("configurationPackage.upload") as HTMLButtonElement).disabled
        ).toBe(false)
        fireEvent.click(screen.getByTestId("package-stop-following"))
        expect(screen.queryByTestId("package-run")).toBeNull()
        expect(sessionStorage.getItem("configurationPackage.active.t1")).toBeNull()
        expect(mockApply).toHaveBeenCalledTimes(1)
    })
})

it("keeps the history fresh: network-backed, refetched after a submission and when a task ends", async () => {
    mockApply.mockResolvedValue(accepted("task-p"))
    const {rerender} = render(<SettingsConfigurationPackage />)
    expect(mockHistoryOptions.mock.calls[0][0].fetchPolicy).toBe("cache-and-network")
    await upload()
    expect(mockRefetch).toHaveBeenCalledTimes(1)
    mockTask = task("task-p", "SUCCESS", record("previewed"))
    rerender(<SettingsConfigurationPackage />)
    expect(mockRefetch).toHaveBeenCalledTimes(2)
})

describe("the SHA-256 entered for an upload", () => {
    const previewWithSha = async (sha: string, recorded: string) => {
        mockSha = sha
        mockApply.mockResolvedValueOnce(accepted("task-p"))
        const view = render(<SettingsConfigurationPackage />)
        await upload()
        const preview = record("previewed") as any
        preview.application.packageSha256 = recorded
        mockTask = task("task-p", "SUCCESS", preview)
        view.rerender(<SettingsConfigurationPackage />)
    }

    it("must be the digest of the package the platform read", async () => {
        await previewWithSha("aa".repeat(32), "bb".repeat(32))
        expect(screen.getByTestId("package-sha-mismatch")).toBeTruthy()
        expect(applyButton().disabled).toBe(true)
    })

    it("allows the application when it matches, in any case", async () => {
        await previewWithSha("AA".repeat(32), "aa".repeat(32))
        expect(screen.queryByTestId("package-sha-mismatch")).toBeNull()
        expect(applyButton().disabled).toBe(false)
    })
})

it("drops a reply that arrives after the tenant changed", async () => {
    let finish: (value: unknown) => void = () => undefined
    mockApply.mockImplementationOnce(() => new Promise((resolve) => (finish = resolve)))
    const {rerender} = render(<SettingsConfigurationPackage />)
    fireEvent.click(screen.getByText("configurationPackage.upload"))
    fireEvent.click(screen.getByTestId("drawer-upload"))
    mockTenant = "t2"
    rerender(<SettingsConfigurationPackage />)
    await act(async () => finish(accepted("task-old")))
    expect(screen.queryByTestId("package-run")).toBeNull()
    expect(sessionStorage.getItem("configurationPackage.active.t1")).toBeNull()
    expect(sessionStorage.getItem("configurationPackage.active.t2")).toBeNull()
})

it("restores the remembered requirements after a reload", () => {
    sessionStorage.setItem(
        "configurationPackage.active.t1",
        JSON.stringify({
            taskId: "task-p",
            mode: "preview",
            documentId: "doc-1",
            requirements: {sections: ["trustees"], census: "optional"},
        })
    )
    render(<SettingsConfigurationPackage />)
    expect((screen.getByLabelText("census") as HTMLInputElement).checked).toBe(false)
    expect((screen.getByLabelText("trustees") as HTMLInputElement).checked).toBe(true)
    expect((screen.getByLabelText("election-event") as HTMLInputElement).checked).toBe(false)
})

it("names a nested member's container", async () => {
    mockApply.mockResolvedValue(accepted("task-p"))
    const {rerender} = render(<SettingsConfigurationPackage />)
    await upload()
    const preview = record("previewed") as any
    preview.application.coverage.members.push({
        container: "official_election_setup.zip",
        path: "export_election_event-e1.json",
        sha256: "22".repeat(32),
        size: 10,
        disposition: {kind: "applied", section: "election-event"},
    })
    mockTask = task("task-p", "SUCCESS", preview)
    rerender(<SettingsConfigurationPackage />)
    expect(screen.getByTestId("package-members").textContent).toContain(
        "official_election_setup.zip › export_election_event-e1.json"
    )
})

it("shows what applying would change before the confirmation, names and keys only", async () => {
    mockApply.mockResolvedValueOnce(accepted("task-p"))
    const {rerender} = render(<SettingsConfigurationPackage />)
    await upload()
    // The changes in configuration_package_sections.rs's own shapes.
    const section = (name: string, changes: unknown) => ({
        section: name,
        members: [],
        status: {state: "previewed", preview: {needsApply: true, changes, blockers: []}},
        history: {submissions: [], applied: false},
    })
    const preview = record("previewed") as any
    preview.application.sections = [
        section("tenant-settings", [
            {field: "displayName", action: "update", from: "Old", to: "New Commission"},
        ]),
        section("role-permissions", [
            {group: "admin", action: "update", add: ["tenant-write"], remove: []},
        ]),
        section("admin-accounts", [
            {username: "operator.one", group: "admin", action: "create", password: "hunter2"},
        ]),
    ]
    mockTask = task("task-p", "SUCCESS", preview)
    rerender(<SettingsConfigurationPackage />)
    fireEvent.click(applyButton())
    const proposed = screen.getByTestId("package-proposed-changes").textContent
    expect(proposed).toContain("displayName · update · Old → New Commission")
    expect(proposed).toContain("admin · update · add: tenant-write")
    expect(proposed).toContain("operator.one / admin · create")
    expect(proposed).not.toContain("hunter2")
    expect(mockApply).toHaveBeenCalledTimes(1)
})

describe("a failed or uncertain submission never brings an old preview's Apply back", () => {
    it.each([
        ["rejected", () => mockApply.mockRejectedValueOnce(new Error("timeout"))],
        [
            "answered with errors",
            () => mockApply.mockResolvedValueOnce({data: null, errors: [{message: "denied"}]}),
        ],
        ["answered without a task", () => mockApply.mockResolvedValueOnce({data: {}})],
    ])("after an apply %s, on a reload", async (_, fail) => {
        const view = await previewed()
        fail()
        await apply()
        expect(applyButton().disabled).toBe(true)
        expect(sessionStorage.getItem("configurationPackage.active.t1")).toBeNull()
        view.unmount()
        render(<SettingsConfigurationPackage />)
        expect(applyButton().disabled).toBe(true)
        expect(screen.queryByTestId("package-run")).toBeNull()
    })

    it("after a rejected apply, on switching tenants away and back", async () => {
        const view = await previewed()
        mockApply.mockRejectedValueOnce(new Error("timeout"))
        await apply()
        mockTenant = "t2"
        view.rerender(<SettingsConfigurationPackage />)
        mockTenant = "t1"
        view.rerender(<SettingsConfigurationPackage />)
        view.rerender(<SettingsConfigurationPackage />)
        expect(applyButton().disabled).toBe(true)
    })

    it("after a reply dropped by a tenant switch", async () => {
        const view = await previewed()
        let finish: (value: unknown) => void = () => undefined
        mockApply.mockImplementationOnce(() => new Promise((resolve) => (finish = resolve)))
        fireEvent.click(applyButton())
        fireEvent.click(screen.getByTestId("package-apply-confirm"))
        mockTenant = "t2"
        view.rerender(<SettingsConfigurationPackage />)
        await act(async () => finish(accepted("task-a")))
        mockTenant = "t1"
        view.rerender(<SettingsConfigurationPackage />)
        view.rerender(<SettingsConfigurationPackage />)
        expect(applyButton().disabled).toBe(true)
    })
})
