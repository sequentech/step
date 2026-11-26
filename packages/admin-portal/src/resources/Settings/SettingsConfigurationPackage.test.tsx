/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, fireEvent, render, screen, waitFor, within} from "@testing-library/react"
import {ThemeProvider} from "@mui/material/styles"
import {adminTheme} from "../../../../ui-essentials/src/services/theme"
import {record, refusedPreview} from "./configurationPackage.fixture"
import {COMPLETE_DELIVERY} from "./configurationPackage"
import {
    notifyConfigurationPackageElectionImported,
    useConfigurationPackageElectionRefresh,
} from "@/hooks/useConfigurationPackageElectionRefresh"
import type {RosterStatus} from "./ArchitectSignerRoster"
import {usePendingConfigurationPackageRefresh} from "@/hooks/usePendingConfigurationPackageRefresh"

// LinkBehavior is not exercised by these controls.
jest.mock("react-router-dom", () => ({Link: "a"}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        disabledButtonStyle: jest.requireActual("../../../../ui-essentials/src/services/theme")
            .disabledButtonStyle,
    }),
    {virtual: true}
)

const mockApply = jest.fn()
const mockManagedApply = jest.fn()
const rosterSha = "a1".repeat(32)
const legacyRoster: RosterStatus = {
    mode: "not_enabled",
    generation: "0",
    active_sha256: null,
    required_approvals: 2,
    can_manage: false,
}
const managedRoster: RosterStatus = {
    ...legacyRoster,
    mode: "portal_managed",
    generation: "1",
    active_sha256: rosterSha,
}
let mockRoster: RosterStatus | null = legacyRoster
const mockRosterRead = jest.fn(async (): Promise<RosterStatus | null> => mockRoster)
jest.mock("./ArchitectSignerRoster", () => ({
    ArchitectSignerRoster: () => null,
    rosterStateAllowsImport: (status: RosterStatus | null) =>
        status?.mode === "not_enabled" || !!status?.active_sha256,
    useArchitectSignerRoster: () => ({
        status: mockRoster,
        loading: false,
        read: mockRosterRead,
    }),
}))
const mockTaskQuery = jest.fn()
const mockMenuPoll = jest.fn(async (_options: unknown) => ({
    data: {sequent_backend_tasks_execution: mockTask ? [mockTask] : []},
}))
const mockApollo = {query: mockMenuPoll}
let mockTask: any = null
let mockAllowed: boolean | ((permission: string) => boolean) = true
let mockHistory: unknown[] = []
let mockActor = "operator-a"
let mockTenant = "t1"
let mockTaskError: unknown = undefined
let mockSha = ""
let mockEncryptedSha: string | undefined
const mockStopPolling = jest.fn()
const mockRefetch = jest.fn(() => Promise.resolve())
const mockHistoryOptions = jest.fn()

// The trust section has its own tests (ConfigurationSigningSettings.test.tsx);
// here it only has to be on the page.
jest.mock("./ArchitectApprovalPolicy", () => ({
    ArchitectApprovalPolicy: () => null,
}))
jest.mock("./ConfigurationSigningSettings", () => ({
    ConfigurationSigningSettings: ({tenantId}: {tenantId: string}) => (
        <div data-testid="configuration-signing">{tenantId}</div>
    ),
}))
jest.mock("@/queries/ApplyConfigurationPackage", () => ({
    APPLY_CONFIGURATION_PACKAGE: "APPLY",
    APPLY_MANAGED_CONFIGURATION_PACKAGE: "MANAGED",
    LIST_PACKAGE_APPLICATIONS: "HISTORY",
    GET_PACKAGE_TASK: "TASK",
}))
jest.mock("@apollo/client", () => ({
    useApolloClient: () => mockApollo,
    useMutation: (op: string) => [op === "MANAGED" ? mockManagedApply : mockApply],
    useQuery: (query: string, options: any) => {
        if (query === "TASK") {
            mockTaskQuery(options)
            return {
                data:
                    options.skip || !mockTask
                        ? undefined
                        : {
                              sequent_backend_tasks_execution:
                                  mockTask === "none" ? [] : [mockTask],
                          },
                error: options.skip ? undefined : mockTaskError,
                loading: false,
                startPolling: jest.fn(),
                stopPolling: mockStopPolling,
            }
        }
        mockHistoryOptions(options)
        return {
            data: {sequent_backend_tasks_execution: mockHistory},
            refetch: mockRefetch,
        }
    },
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
    }),
}))
jest.mock("@/providers/TenantContextProvider", () => ({
    useTenantStore: () => [mockTenant],
}))
jest.mock("@/providers/AuthContextProvider", () => {
    const React = require("react")
    return {
        AuthContext: React.createContext({
            isAuthorized: (_superAdmin: boolean, _tenant: string | null, permission: string) =>
                typeof mockAllowed === "function" ? mockAllowed(permission) : mockAllowed,
            get userId() {
                return mockActor
            },
        }),
    }
})
// The real drawer and screen are exercised in SettingsConfigurationPackage.upload.test.tsx.
jest.mock("@/components/election-event/import-data/ImportScreen", () => ({
    ImportScreen: ({doImport}: any) => (
        <button
            onClick={() => doImport("doc-1", mockSha, undefined, mockEncryptedSha)}
            data-testid="drawer-upload"
        >
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
const accepted = (id: string) => ({
    data: {apply_configuration_package: {task_execution: {id}}},
})
const applyButton = () => screen.getByTestId("package-apply") as HTMLButtonElement

const showAdvanced = () => {
    const details = screen.getByTestId("package-advanced-details") as HTMLDetailsElement
    const summary = details.querySelector("summary")
    if (!summary) throw new Error("Advanced options has no disclosure")
    if (!details.open) fireEvent.click(summary)
    expect(details.open).toBe(true)
    return {details, summary}
}

beforeEach(() => {
    jest.clearAllMocks()
    sessionStorage.clear()
    mockTask = null
    mockAllowed = true
    mockHistory = []
    mockActor = "operator-a"
    mockTenant = "t1"
    mockTaskError = undefined
    mockSha = ""
    mockEncryptedSha = undefined
    mockRoster = legacyRoster
    mockRosterRead.mockImplementation(async () => mockRoster)
})

/** Click Apply and confirm it: the application is never one click. */
const apply = async () => {
    fireEvent.click(applyButton())
    await act(async () => {
        fireEvent.click(screen.getByTestId("package-apply-confirm"))
    })
}

const upload = async () => {
    fireEvent.click(screen.getByTestId("package-upload"))
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
    expect(screen.queryByTestId("package-upload")).toBeNull()
    expect(screen.queryByTestId("package-advanced")).toBeNull()
    expect(screen.queryByTestId("configuration-signing")).toBeNull()
})

it("offers one initial next action and summarizes the closed advanced options without submitting", () => {
    render(<SettingsConfigurationPackage />)
    const workflow = screen.getByTestId("package-workflow")
    expect(
        within(workflow).getByRole("heading", {
            name: "configurationPackage.workflow.phases.choose.title",
        })
    ).toBeTruthy()
    expect(
        screen.getByTestId("package-run-stages").querySelector('[aria-current="step"]')?.textContent
    ).toContain("configurationPackage.hub.stages.package")
    expect(screen.queryByRole("button", {name: "configurationPackage.apply"})).toBeNull()
    expect(
        screen.getByTestId("package-advanced-details").querySelector("summary")?.textContent
    ).toContain("configurationPackage.workflow.censusRequired")
    expect(applyButton().disabled).toBe(true)
    expect(mockApply).not.toHaveBeenCalled()
})

it("never signals Review after changing an already-previewed requirement", async () => {
    await previewed()
    const workflow = screen.getByTestId("package-workflow")
    expect(
        within(workflow).getByRole("heading", {
            name: "configurationPackage.workflow.phases.review.title",
        })
    ).toBeTruthy()
    showAdvanced()
    fireEvent.click(screen.getByRole("checkbox", {name: "census"}))
    expect(
        within(workflow).getByRole("heading", {
            name: "configurationPackage.workflow.phases.previewAgain.title",
        })
    ).toBeTruthy()
    expect(applyButton().disabled).toBe(true)
    expect(mockApply.mock.calls.map(([options]) => options.variables.mode)).toEqual(["preview"])
})

it("discloses advanced controls on demand without changing requirements, including legacy setup", () => {
    render(<SettingsConfigurationPackage />)
    const upload = screen.getByTestId("package-upload")
    const advanced = screen.getByRole("region", {
        name: "configurationPackage.advanced",
    })
    expect(upload.compareDocumentPosition(advanced) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect((screen.getByTestId("package-advanced-details") as HTMLDetailsElement).open).toBe(false)
    const {summary} = showAdvanced()
    expect(screen.getByRole("checkbox", {name: "census"})).toBeTruthy()
    expect(screen.queryByTestId("configuration-signing")).toBeNull()
    fireEvent.click(screen.getByRole("checkbox", {name: "census"}))
    fireEvent.click(summary)
    expect(
        within(advanced).getByText(/configurationPackage\.workflow\.censusOptional/)
    ).toBeTruthy()
    showAdvanced()
    expect((screen.getByRole("checkbox", {name: "census"}) as HTMLInputElement).checked).toBe(false)
    const setup = screen.getByRole("button", {
        name: "configurationPackage.legacySetup",
    })
    expect(setup.getAttribute("aria-expanded")).toBe("false")
    fireEvent.click(setup)
    expect(setup.getAttribute("aria-expanded")).toBe("true")
    const editor = screen.getByTestId("configuration-signing")
    expect(editor.textContent).toBe("t1")
    expect(
        document.getElementById(setup.getAttribute("aria-controls") ?? "")?.contains(editor)
    ).toBe(true)
    expect(mockApply).not.toHaveBeenCalled()
})

it("shows the recorded FAILED/refused preview blocker without allowing Apply", async () => {
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

    mockTask = task("task-p", "FAILED", refusedPreview(["unknown role x"]))
    rerender(<SettingsConfigurationPackage />)
    expect(screen.getByTestId("package-task").textContent).toContain("task-p")
    expect(screen.getByTestId("package-members").textContent).toContain("export_voters-e1.csv")
    expect(
        within(screen.getByTestId("package-workflow")).getByRole("heading", {
            name: "configurationPackage.workflow.phases.previewBlocked.title",
        })
    ).toBeTruthy()
    expect(screen.getByTestId("package-blocked").textContent).toContain("unknown role x")
    fireEvent.click(applyButton())
    expect(screen.queryByTestId("package-apply-confirm")).toBeNull()
    expect(mockApply.mock.calls.map(([options]) => options.variables.mode)).toEqual(["preview"])
    expect(mockManagedApply).not.toHaveBeenCalled()
    expect(screen.getByTestId("section-tenant-configuration").textContent).toContain(
        "unknown role x"
    )
    expect(applyButton().disabled).toBe(true)
    expect(mockStopPolling).toHaveBeenCalled()
})

it("names missing required sections and census even without a recorded blocker string", async () => {
    mockApply.mockResolvedValue(accepted("task-p"))
    const {rerender} = render(<SettingsConfigurationPackage />)
    await upload()
    mockTask = task(
        "task-p",
        "FAILED",
        refusedPreview([], {missingTrustees: true, missingCensus: true})
    )
    rerender(<SettingsConfigurationPackage />)
    const alert = screen.getByTestId("package-blocked")
    expect(alert.textContent).toContain("configurationPackage.workflow.missingSection")
    expect(alert.textContent).toContain("configurationPackage.sections.trustees")
    expect(alert.textContent).toContain("configurationPackage.workflow.missingCensus")
    expect(screen.getByTestId("section-trustees").textContent).toContain("missing")
    expect(applyButton().disabled).toBe(true)
    expect(mockApply).toHaveBeenCalledTimes(1)
})

it("does not use a different task's refused preview as the current blocker", async () => {
    mockApply.mockResolvedValue(accepted("current-task"))
    const {rerender} = render(<SettingsConfigurationPackage />)
    await upload()
    mockTask = task("other-task", "FAILED", refusedPreview(["stale blocker"]))
    rerender(<SettingsConfigurationPackage />)
    expect(screen.queryByTestId("package-blocked")).toBeNull()
    expect(applyButton().disabled).toBe(true)
    expect(mockApply).toHaveBeenCalledTimes(1)
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
    expect(mockApply.mock.calls[1][0].variables).toMatchObject({
        documentId: "doc-1",
        mode: "apply",
    })
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

    mockApply.mockResolvedValueOnce({
        data: null,
        errors: [{message: "missing permission"}],
    })
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
    expect(mockTaskQuery.mock.calls[0][0].variables).toEqual({
        taskId: "task-p",
        tenantId: "t1",
    })
    expect(screen.getByTestId("package-task").textContent).toContain("task-p")
    // The resumed preview may be applied, for the same document and requirements.
    expect(applyButton().disabled).toBe(false)
})

it("clears the preview approval when the requirements change", async () => {
    await previewed()
    expect(applyButton().disabled).toBe(false)
    showAdvanced()
    fireEvent.click(screen.getByLabelText("census"))
    expect(applyButton().disabled).toBe(true)
    // Changing them back still needs a new preview.
    fireEvent.click(screen.getByLabelText("census"))
    expect(applyButton().disabled).toBe(true)
})

it("clears the preview approval when the document changes", async () => {
    await previewed()
    expect(applyButton().disabled).toBe(false)
    showAdvanced()
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
    showAdvanced()
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

const originalRetryRequirements = {
    sections: [
        "tenant-configuration",
        "tenant-settings",
        "role-permissions",
        "admin-accounts",
        "messaging-accounts",
        "election-event",
    ],
    census: "required",
}

const retryNotice = {
    tenantId: "t1",
    packageSha256: "e7".repeat(32),
    requirements: originalRetryRequirements,
}

const retryButton = () =>
    screen.queryByRole("button", {name: "configurationPackage.retryRequirements.preview"})

it("previews the original six requirements with the same protected upload only on an explicit click", async () => {
    mockEncryptedSha = retryNotice.packageSha256
    mockApply.mockResolvedValueOnce(accepted("changed-options"))
    const {rerender} = render(<SettingsConfigurationPackage />)
    for (const section of COMPLETE_DELIVERY.sections.filter(
        (value) => value !== "election-event"
    )) {
        fireEvent.click(screen.getByRole("checkbox", {name: section}))
    }
    await upload()
    expect(mockApply.mock.calls[0][0].variables.requirements.sections).toEqual(["election-event"])
    mockTask = task("changed-options", "FAILED", {packageRetry: retryNotice})
    rerender(<SettingsConfigurationPackage />)
    expect(screen.getByTestId("package-retry-outcome").textContent).toContain(
        "configurationPackage.retryRequirements.refused"
    )
    expect(screen.getByTestId("package-retry-requirements").textContent).toContain(
        "configurationPackage.sections.messaging-accounts"
    )
    expect(mockApply).toHaveBeenCalledTimes(1) // polling never restarts an import
    expect(applyButton().disabled).toBe(true)
    mockApply.mockResolvedValueOnce(accepted("fresh-preview"))
    await act(async () => fireEvent.click(retryButton()!))
    expect(mockApply).toHaveBeenCalledTimes(2)
    expect(mockApply.mock.calls[1][0].variables).toEqual({
        tenantId: "t1",
        documentId: "doc-1",
        mode: "preview",
        requirements: originalRetryRequirements,
        encryptedPayloadSha256: retryNotice.packageSha256,
    })
    expect(mockManagedApply).not.toHaveBeenCalled()
    showAdvanced()
    expect((screen.getByRole("checkbox", {name: "trustees"}) as HTMLInputElement).checked).toBe(
        false
    )
    expect((screen.getByRole("checkbox", {name: "census"}) as HTMLInputElement).checked).toBe(true)
    expect(applyButton().disabled).toBe(true)
})

it("will not apply if the re-preview read different bytes from the same document", async () => {
    mockApply.mockResolvedValueOnce(accepted("changed-options"))
    const {rerender} = render(<SettingsConfigurationPackage />)
    await upload()
    mockTask = task("changed-options", "FAILED", {packageRetry: retryNotice})
    rerender(<SettingsConfigurationPackage />)
    mockApply.mockResolvedValueOnce(accepted("fresh-preview"))
    await act(async () => fireEvent.click(retryButton()!))
    const changed = record("previewed")
    changed.application.packageSha256 = "aa".repeat(32)
    mockTask = task("fresh-preview", "SUCCESS", changed)
    rerender(<SettingsConfigurationPackage />)
    expect(screen.getByTestId("package-sha-mismatch")).toBeTruthy()
    expect(applyButton().disabled).toBe(true)
    expect(mockApply).toHaveBeenCalledTimes(2)
    expect(mockManagedApply).not.toHaveBeenCalled()
})

it.each([
    ["a different tenant", {tenantId: "other"}],
    ["different bytes", {packageSha256: "ab".repeat(32)}],
    ["bad saved options", {requirements: {sections: ["not-a-section"], census: "required"}}],
])("does not offer a requirements retry for %s", async (_, override) => {
    mockEncryptedSha = retryNotice.packageSha256
    mockApply.mockResolvedValueOnce(accepted("changed-options"))
    const {rerender} = render(<SettingsConfigurationPackage />)
    await upload()
    mockTask = task("changed-options", "FAILED", {packageRetry: {...retryNotice, ...override}})
    rerender(<SettingsConfigurationPackage />)
    expect(retryButton()).toBeNull()
    expect(applyButton().disabled).toBe(true)
    expect(mockApply).toHaveBeenCalledTimes(1)
})

it("does not offer the retry on a concurrent, completed, cancelled or different task", async () => {
    mockApply.mockResolvedValueOnce(accepted("changed-options"))
    const {rerender} = render(<SettingsConfigurationPackage />)
    await upload()
    for (const current of [
        task("changed-options", "IN_PROGRESS", {packageRetry: retryNotice}),
        task("changed-options", "CANCELLED", {packageRetry: retryNotice}),
        task("changed-options", "SUCCESS", {...record("applied"), packageRetry: retryNotice}),
        task("changed-options", "FAILED", {previouslyApplied: {}, packageRetry: retryNotice}),
        task("other-task", "FAILED", {packageRetry: retryNotice}),
    ]) {
        mockTask = current
        rerender(<SettingsConfigurationPackage />)
        expect(retryButton()).toBeNull()
        expect(applyButton().disabled).toBe(true)
    }
    expect(mockApply).toHaveBeenCalledTimes(1)
})

it("does not allow retry when roster trust, authority or selected document changes", async () => {
    mockApply.mockResolvedValueOnce(accepted("changed-options"))
    const {rerender} = render(<SettingsConfigurationPackage />)
    await upload()
    mockTask = task("changed-options", "FAILED", {packageRetry: retryNotice})
    rerender(<SettingsConfigurationPackage />)
    expect(retryButton()).toBeTruthy()
    showAdvanced()
    const entry = screen.getByRole("textbox", {name: "configurationPackage.documentId"})
    fireEvent.change(entry, {target: {value: "different-document"}})
    expect(retryButton()).toBeNull()
    fireEvent.change(entry, {target: {value: "doc-1"}})
    mockRoster = {...managedRoster, active_sha256: null}
    rerender(<SettingsConfigurationPackage />)
    expect((retryButton() as HTMLButtonElement).disabled).toBe(true)
    mockAllowed = false
    rerender(<SettingsConfigurationPackage />)
    expect(retryButton()).toBeNull()
    mockActor = "operator-b"
    mockAllowed = true
    mockRoster = legacyRoster
    rerender(<SettingsConfigurationPackage />)
    expect(retryButton()).toBeNull()
    expect(mockApply).toHaveBeenCalledTimes(1)
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
    expect((screen.getByTestId("package-upload") as HTMLButtonElement).disabled).toBe(false)
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
        expect((screen.getByTestId("package-upload") as HTMLButtonElement).disabled).toBe(false)
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
    fireEvent.click(screen.getByTestId("package-upload"))
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
        status: {
            state: "previewed",
            preview: {needsApply: true, changes, blockers: []},
        },
        history: {submissions: [], applied: false},
    })
    const preview = record("previewed") as any
    preview.application.sections = [
        section("tenant-settings", [
            {
                field: "displayName",
                action: "update",
                from: "Old",
                to: "New Commission",
            },
        ]),
        section("role-permissions", [
            {group: "admin", action: "update", add: ["tenant-write"], remove: []},
        ]),
        section("admin-accounts", [
            {
                username: "operator.one",
                group: "admin",
                action: "create",
                password: "hunter2",
            },
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
            () =>
                mockApply.mockResolvedValueOnce({
                    data: null,
                    errors: [{message: "denied"}],
                }),
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

const finishManagedPreview = (overrides: Record<string, unknown> = {}) => {
    const annotated = record("previewed")
    Object.assign(annotated.application, {
        managedRosterBinding: {
            sha256: rosterSha,
            generation: 1,
            requiredApprovals: 2,
        },
        managedRequirements: COMPLETE_DELIVERY,
        ...overrides,
    })
    mockTask = task("task-p", "SUCCESS", annotated)
}

it("uses the exact successful server-bound Preview only after managed Apply confirmation", async () => {
    mockRoster = managedRoster
    const view = await previewed()
    expect(applyButton().disabled).toBe(true)
    finishManagedPreview()
    view.rerender(<SettingsConfigurationPackage />)
    expect(applyButton().disabled).toBe(false)
    fireEvent.click(applyButton())
    expect(mockManagedApply).not.toHaveBeenCalled()
    mockManagedApply.mockResolvedValueOnce(accepted("task-a"))
    await act(async () => {
        const confirm = screen.getByTestId("package-apply-confirm")
        fireEvent.click(confirm)
        fireEvent.click(confirm)
    })
    expect(mockManagedApply).toHaveBeenCalledTimes(1)
    expect(mockManagedApply).toHaveBeenCalledWith({
        variables: {
            tenantId: "t1",
            documentId: "doc-1",
            requirements: COMPLETE_DELIVERY,
            previewTaskId: "task-p",
        },
    })
    expect(mockApply).toHaveBeenCalledTimes(1)
})

const finishPortablePreview = (sha: string | null = "a5".repeat(32), changes?: unknown[]) => {
    const annotated = record("previewed")
    annotated.application.coverage = {
        ...annotated.application.coverage,
        sections: {
            ...annotated.application.coverage.sections,
            ...({
                "messaging-accounts": {
                    state: "present",
                    members: ["tenant_messaging_accounts.json"],
                },
            } as Record<string, {state: string; members: string[]}>),
        },
    }
    Object.assign(annotated.application, {
        managedRequirements: COMPLETE_DELIVERY,
        ...(sha === null ? {} : {messagingPlanSha256: sha}),
    })
    if (changes) {
        annotated.application.sections.push({
            section: "messaging-accounts",
            members: ["tenant_messaging_accounts.json"],
            status: {state: "previewed", preview: {needsApply: true, changes, blockers: []}},
        })
    }
    mockTask = task("task-p", "SUCCESS", annotated)
}

it.each(["not_enabled", "site_pinned"] as const)(
    "uses the exact successful Preview for portable messaging in %s mode",
    async (mode) => {
        mockRoster = {
            ...legacyRoster,
            mode,
            active_sha256: mode === "site_pinned" ? rosterSha : null,
        }
        const view = await previewed()
        finishPortablePreview()
        view.rerender(<SettingsConfigurationPackage />)
        expect(applyButton().disabled).toBe(false)
        mockManagedApply.mockResolvedValueOnce(accepted("task-a"))
        await apply()
        expect(mockManagedApply).toHaveBeenCalledWith({
            variables: {
                tenantId: "t1",
                documentId: "doc-1",
                requirements: COMPLETE_DELIVERY,
                previewTaskId: "task-p",
            },
        })
        expect(mockManagedApply).toHaveBeenCalledTimes(1)
        expect(mockApply).toHaveBeenCalledTimes(1)
        expect(mockRosterRead).not.toHaveBeenCalled()
    }
)

it("warns about a portable replacement in Preview and explicit Apply confirmation", async () => {
    const view = await previewed()
    finishPortablePreview(undefined, [
        {reference: "package:ses", action: "replace", channel: "EMAIL", provider: "AWS_SES"},
    ])
    view.rerender(<SettingsConfigurationPackage />)
    const warning = "configurationPackage.messagingReplaceWarning"
    expect(within(screen.getByTestId("section-messaging-accounts")).getByText(warning)).toBeTruthy()
    expect(applyButton().disabled).toBe(false)
    expect(mockManagedApply).not.toHaveBeenCalled()
    await act(async () => fireEvent.click(applyButton()))
    expect(within(screen.getByRole("dialog")).getByText(warning)).toBeTruthy()
    expect(mockManagedApply).not.toHaveBeenCalled()
    expect(mockApply).toHaveBeenCalledTimes(1)
})

it("fails before Apply when a portable Preview has no server recipient-plan receipt", async () => {
    const view = await previewed()
    finishPortablePreview(null)
    view.rerender(<SettingsConfigurationPackage />)
    expect(applyButton().disabled).toBe(true)
    expect(mockManagedApply).not.toHaveBeenCalled()
    expect(mockApply).toHaveBeenCalledTimes(1)
})

it("does not reuse another actor's portable Preview", async () => {
    const view = await previewed()
    finishPortablePreview()
    view.rerender(<SettingsConfigurationPackage />)
    expect(applyButton().disabled).toBe(false)
    mockActor = "operator-b"
    view.rerender(<SettingsConfigurationPackage />)
    expect(applyButton().disabled).toBe(true)
    expect(mockManagedApply).not.toHaveBeenCalled()
})

it("drops a managed Preview when confirmation refetch observes a rotated roster", async () => {
    mockRoster = managedRoster
    const view = await previewed()
    finishManagedPreview()
    view.rerender(<SettingsConfigurationPackage />)
    mockRosterRead.mockResolvedValueOnce({
        ...managedRoster,
        generation: "2",
        active_sha256: "b2".repeat(32),
    })
    await apply()
    expect(mockManagedApply).not.toHaveBeenCalled()
    expect(mockApply).toHaveBeenCalledTimes(1)
    expect(screen.getByTestId("package-error").textContent).toBe(
        "configurationPackage.roster.previewAgain"
    )
    expect(screen.queryByTestId("package-run")).toBeNull()
})

it("does not consider an old trust binding applicable after observing another generation", async () => {
    mockRoster = managedRoster
    const view = await previewed()
    finishManagedPreview()
    view.rerender(<SettingsConfigurationPackage />)
    expect(applyButton().disabled).toBe(false)
    mockRoster = {...managedRoster, generation: "2"}
    view.rerender(<SettingsConfigurationPackage />)
    expect(applyButton().disabled).toBe(true)
    expect(screen.getByText("configurationPackage.roster.previewAgain")).toBeTruthy()
    expect(mockManagedApply).not.toHaveBeenCalled()
})

it.each([null, {...managedRoster, generation: "0", active_sha256: null}])(
    "blocks starting a package upload until receiver trust is ready (%p)",
    (status) => {
        mockRoster = status
        render(<SettingsConfigurationPackage />)
        expect((screen.getByTestId("package-upload") as HTMLButtonElement).disabled).toBe(true)
        expect(mockApply).not.toHaveBeenCalled()
    }
)

it("keeps imported ZIP history readable but will not apply a restored managed preview", async () => {
    mockRoster = managedRoster
    const view = await previewed()
    finishManagedPreview()
    view.rerender(<SettingsConfigurationPackage />)
    expect(applyButton().disabled).toBe(false)
    view.unmount()
    render(<SettingsConfigurationPackage />)
    expect(screen.getByTestId("package-task").textContent).toContain("task-p")
    expect(applyButton().disabled).toBe(true)
    expect(mockManagedApply).not.toHaveBeenCalled()
})

it("does not reuse a successful managed Preview when the signed-in actor switches in the same tenant", async () => {
    mockRoster = managedRoster
    const view = await previewed()
    finishManagedPreview()
    view.rerender(<SettingsConfigurationPackage />)
    expect(applyButton().disabled).toBe(false)
    mockActor = "operator-b"
    view.rerender(<SettingsConfigurationPackage />)
    expect(applyButton().disabled).toBe(true)
    expect(mockManagedApply).not.toHaveBeenCalled()
    mockActor = ""
    view.rerender(<SettingsConfigurationPackage />)
    expect(screen.getByTestId("package-not-permitted")).toBeTruthy()
})

const expectDisabledLikeApply = (upload: HTMLButtonElement) => {
    expect(upload.disabled).toBe(true)
    expect(upload.classList.contains("Mui-disabled")).toBe(true)
    const uploadStyle = window.getComputedStyle(upload)
    const reference = window.getComputedStyle(applyButton())
    expect(applyButton().disabled).toBe(true)
    expect([uploadStyle.backgroundColor, uploadStyle.color, uploadStyle.border]).toEqual([
        reference.backgroundColor,
        reference.color,
        reference.border,
    ])
    expect(uploadStyle.backgroundColor).toBe("rgba(15, 5, 76, 0.4)")
}

it("keeps managed trust roster-only and unlocks ZIP/document selection only after active status", async () => {
    mockRoster = {...managedRoster, generation: "0", active_sha256: null}
    const page = () => (
        <ThemeProvider theme={adminTheme}>
            <SettingsConfigurationPackage />
        </ThemeProvider>
    )
    const view = render(page())
    const upload = screen.getByTestId("package-upload") as HTMLButtonElement
    expect(upload.disabled).toBe(true)
    expectDisabledLikeApply(upload)
    expect(upload.getAttribute("aria-describedby")).toBeTruthy()
    fireEvent.click(upload)
    expect(screen.queryByTestId("package-upload-drawer")).toBeNull()
    showAdvanced()
    const documentId = screen.getByRole("textbox", {
        name: "configurationPackage.signedDocumentId",
    }) as HTMLInputElement
    expect(documentId.disabled).toBe(true)
    expect(screen.getByRole("checkbox", {name: "census"})).toBeTruthy()
    expect(screen.queryByText("configurationPackage.legacySetup")).toBeNull()
    expect(screen.queryByTestId("configuration-signing")).toBeNull()

    mockRoster = managedRoster
    view.rerender(page())
    expect(upload.disabled).toBe(false)
    expect(documentId.disabled).toBe(false)
    expect(screen.queryByText("configurationPackage.legacySetup")).toBeNull()
    fireEvent.change(documentId, {target: {value: "existing-zip"}})
    mockApply.mockResolvedValueOnce(accepted("task-doc"))
    await act(async () =>
        fireEvent.click(screen.getByRole("button", {name: "configurationPackage.preview"}))
    )
    expect(mockApply).toHaveBeenCalledWith({
        variables: {
            tenantId: "t1",
            documentId: "existing-zip",
            mode: "preview",
            requirements: COMPLETE_DELIVERY,
        },
    })
    expect(mockManagedApply).not.toHaveBeenCalled()
    // A running preview uses the same existing disabled style as pending trust.
    expectDisabledLikeApply(upload)
})

it("never flashes a legacy trust or document choice while receiver status is unknown", () => {
    mockRoster = null
    const view = render(<SettingsConfigurationPackage />)
    expect(screen.getByRole("region", {name: "configurationPackage.advanced"})).toBeTruthy()
    expect(screen.queryByText("configurationPackage.legacySetup")).toBeNull()
    expect(screen.queryByRole("textbox", {name: "configurationPackage.documentId"})).toBeNull()
    mockRoster = {...legacyRoster, mode: "unavailable"}
    view.rerender(<SettingsConfigurationPackage />)
    expect(screen.queryByText("configurationPackage.legacySetup")).toBeNull()
})

const mockElectionRefresh = jest.fn()
function ScreenWithElectionRefresh() {
    useConfigurationPackageElectionRefresh(mockElectionRefresh)
    return <SettingsConfigurationPackage />
}

it("refreshes elections only once for the successful matching Apply, not upload, preview, queued or failed tasks", async () => {
    mockApply.mockResolvedValueOnce(accepted("task-p"))
    const ui = render(<ScreenWithElectionRefresh />)
    await upload()
    expect(mockElectionRefresh).not.toHaveBeenCalled()
    mockTask = task("task-p", "SUCCESS", record("previewed"))
    ui.rerender(<ScreenWithElectionRefresh />)
    expect(mockElectionRefresh).not.toHaveBeenCalled()
    mockApply.mockResolvedValueOnce(accepted("task-a"))
    await apply()
    for (const row of [
        task("old-unrelated", "SUCCESS", record("applied")),
        task("task-a", "IN_PROGRESS", null),
        task("task-a", "FAILED", record("failed")),
        task("task-a", "SUCCESS", record("refused")),
    ]) {
        mockTask = row
        ui.rerender(<ScreenWithElectionRefresh />)
        expect(mockElectionRefresh).not.toHaveBeenCalled()
    }
    mockTask = task("task-a", "SUCCESS", record("applied"))
    ui.rerender(<ScreenWithElectionRefresh />)
    expect(mockElectionRefresh).toHaveBeenCalledTimes(1)
    mockTask = task("task-a", "SUCCESS", record("applied"))
    ui.rerender(<ScreenWithElectionRefresh />)
    expect(mockElectionRefresh).toHaveBeenCalledTimes(1)
    ui.unmount()
    render(<ScreenWithElectionRefresh />)
    expect(mockElectionRefresh).toHaveBeenCalledTimes(1)
})

it("does not refresh another tenant or actor, or replay another operator's recalled Apply", async () => {
    mockApply.mockResolvedValueOnce(accepted("task-p"))
    const ui = render(<ScreenWithElectionRefresh />)
    await upload()
    mockTask = task("task-p", "SUCCESS", record("previewed"))
    ui.rerender(<ScreenWithElectionRefresh />)
    mockApply.mockResolvedValueOnce(accepted("task-a"))
    await apply()
    mockTask = task("task-a", "SUCCESS", record("applied"))
    mockTenant = "t2"
    ui.rerender(<ScreenWithElectionRefresh />)
    notifyConfigurationPackageElectionImported({
        tenantId: "t1",
        actor: "operator-a",
    })
    expect(mockElectionRefresh).not.toHaveBeenCalled()
    mockTenant = "t1"
    mockActor = "operator-b"
    ui.rerender(<ScreenWithElectionRefresh />)
    notifyConfigurationPackageElectionImported({
        tenantId: "t1",
        actor: "operator-a",
    })
    expect(mockElectionRefresh).not.toHaveBeenCalled()
    mockActor = "operator-a"
    ui.rerender(<ScreenWithElectionRefresh />)
    expect(mockElectionRefresh).toHaveBeenCalledTimes(1)
})

function PersistentNavigation({settings}: {settings: boolean}) {
    usePendingConfigurationPackageRefresh()
    useConfigurationPackageElectionRefresh(mockElectionRefresh)
    return settings ? <SettingsConfigurationPackage /> : <div>Another route</div>
}

it("watches the real accepted Apply in persistent navigation after Settings unmounts, only in the operator scope", async () => {
    jest.useFakeTimers()
    try {
        mockApply.mockResolvedValueOnce(accepted("task-p"))
        const ui = render(<PersistentNavigation settings />)
        expect(mockMenuPoll).not.toHaveBeenCalled()
        await upload()
        mockTask = task("task-p", "SUCCESS", record("previewed"))
        ui.rerender(<PersistentNavigation settings />)
        expect(mockMenuPoll).not.toHaveBeenCalled()
        mockApply.mockResolvedValueOnce(accepted("task-a"))
        await apply()
        expect(mockElectionRefresh).not.toHaveBeenCalled()
        expect(mockMenuPoll).toHaveBeenCalledTimes(1)

        ui.rerender(<PersistentNavigation settings={false} />)
        expect(screen.getByText("Another route")).toBeTruthy()
        mockTask = task("task-a", "SUCCESS", record("applied"))
        mockTenant = "other-tenant"
        ui.rerender(<PersistentNavigation settings={false} />)
        await act(async () => jest.advanceTimersByTime(10000))
        expect(mockMenuPoll).toHaveBeenCalledTimes(1)
        expect(mockElectionRefresh).not.toHaveBeenCalled()

        mockTenant = "t1"
        mockActor = "different-operator"
        ui.rerender(<PersistentNavigation settings={false} />)
        await act(async () => jest.advanceTimersByTime(10000))
        expect(mockMenuPoll).toHaveBeenCalledTimes(1)
        expect(mockElectionRefresh).not.toHaveBeenCalled()

        mockActor = "operator-a"
        await act(async () => {
            ui.rerender(<PersistentNavigation settings={false} />)
        })
        expect(mockMenuPoll).toHaveBeenCalledTimes(2)
        expect(mockMenuPoll.mock.calls.at(-1)?.[0]).toMatchObject({
            variables: {tenantId: "t1", taskId: "task-a"},
        })
        expect(mockElectionRefresh).toHaveBeenCalledTimes(1)
        await act(async () => jest.advanceTimersByTime(15000))
        ui.unmount()
        render(<PersistentNavigation settings={false} />)
        expect(mockMenuPoll).toHaveBeenCalledTimes(2)
        expect(mockElectionRefresh).toHaveBeenCalledTimes(1)
    } finally {
        jest.useRealTimers()
    }
})

it("drops an in-flight successful Apply reply after the operator changes routes and identity", async () => {
    let respond!: (value: {data: {sequent_backend_tasks_execution: unknown[]}}) => void
    mockMenuPoll.mockImplementationOnce(
        () =>
            new Promise((resolve) => {
                respond = resolve
            })
    )
    mockApply.mockResolvedValueOnce(accepted("task-p"))
    const ui = render(<PersistentNavigation settings />)
    await upload()
    mockTask = task("task-p", "SUCCESS", record("previewed"))
    ui.rerender(<PersistentNavigation settings />)
    mockApply.mockResolvedValueOnce(accepted("task-a"))
    await apply()
    expect(mockMenuPoll).toHaveBeenCalledTimes(1)
    mockActor = "operator-b"
    ui.rerender(<PersistentNavigation settings={false} />)
    const result = task("task-a", "SUCCESS", record("applied"))
    await act(async () => {
        respond({data: {sequent_backend_tasks_execution: [result]}})
    })
    expect(mockElectionRefresh).not.toHaveBeenCalled()
    expect(mockMenuPoll).toHaveBeenCalledTimes(1)
    mockActor = "operator-a"
    mockTask = result
    await act(async () => {
        ui.rerender(<PersistentNavigation settings={false} />)
    })
    expect(mockElectionRefresh).toHaveBeenCalledTimes(1)
})

describe("configuration package hub layout", () => {
    it("names exactly the grants the operator lacks, without page controls", () => {
        mockAllowed = (permission) => permission !== "trustee-create" && permission !== "role-write"
        render(<SettingsConfigurationPackage />)
        const missing = screen.getByTestId("package-missing-permissions").textContent ?? ""
        expect(missing).toContain("role-write, trustee-create")
        expect(missing).not.toContain("tenant-write")
        expect(screen.queryByTestId("package-upload")).toBeNull()
        expect(screen.queryByTestId("package-rail")).toBeNull()
    })

    it("rails Overview, the seven sections in apply order, Signing trust and History", () => {
        render(<SettingsConfigurationPackage />)
        const rail = screen.getByTestId("package-rail")
        const ids = Array.from(rail.querySelectorAll("[data-testid^='package-rail-']")).map(
            (item) => item.getAttribute("data-testid")
        )
        expect(ids).toEqual([
            "package-rail-overview",
            "package-rail-tenant-configuration",
            "package-rail-tenant-settings",
            "package-rail-role-permissions",
            "package-rail-admin-accounts",
            "package-rail-trustees",
            "package-rail-messaging-accounts",
            "package-rail-election-event",
            "package-rail-trust",
            "package-rail-history",
        ])
        expect(screen.getByTestId("package-rail-overview").getAttribute("aria-current")).toBe(
            "page"
        )
        fireEvent.click(screen.getByTestId("package-rail-trustees"))
        expect(screen.getByTestId("package-section-panel-trustees")).toBeTruthy()
        expect(screen.getByTestId("package-rail-trustees").getAttribute("aria-current")).toBe(
            "page"
        )
        fireEvent.click(screen.getByTestId("package-rail-trust"))
        expect(screen.getByTestId("package-trust-panel")).toBeTruthy()
        fireEvent.click(screen.getByTestId("package-rail-history"))
        expect(screen.getByTestId("package-history")).toBeTruthy()
    })

    it("offers Review and apply for a ready preview, behind the confirmation", async () => {
        await previewed()
        expect(applyButton().textContent).toBe("configurationPackage.hub.actions.reviewAndApply")
        expect(applyButton().disabled).toBe(false)
        expect(applyButton().hidden).toBe(false)
        fireEvent.click(applyButton())
        expect(screen.getByTestId("package-apply-confirm")).toBeTruthy()
        expect(mockApply.mock.calls.map(([options]) => options.variables.mode)).toEqual(["preview"])
    })

    it("offers only Import fixed package and Change requirements for a blocked preview", async () => {
        mockApply.mockResolvedValue(accepted("task-p"))
        const {rerender} = render(<SettingsConfigurationPackage />)
        await upload()
        mockTask = task(
            "task-p",
            "FAILED",
            refusedPreview(["unknown role x"], {missingTrustees: true})
        )
        rerender(<SettingsConfigurationPackage />)
        expect(screen.getByTestId("package-upload").textContent).toBe(
            "configurationPackage.hub.actions.importFixed"
        )
        expect(screen.getByTestId("package-change-requirements")).toBeTruthy()
        expect(applyButton().hidden).toBe(true)
        expect(applyButton().disabled).toBe(true)
        // The recorded reasons come before the section list, so a phone stacks them first.
        expect(
            screen
                .getByTestId("package-blocked")
                .compareDocumentPosition(screen.getByTestId("package-section-rows")) &
                Node.DOCUMENT_POSITION_FOLLOWING
        ).toBeTruthy()
        expect(
            screen
                .getByTestId("package-run-card")
                .compareDocumentPosition(screen.getByTestId("package-receipt")) &
                Node.DOCUMENT_POSITION_FOLLOWING
        ).toBeTruthy()
        const badge = (section: string) =>
            screen
                .getByTestId(`package-section-row-${section}`)
                .querySelector("[data-badge]")
                ?.getAttribute("data-badge")
        // Blocked and missing rows are folded into the ordered section rows.
        expect(badge("tenant-configuration")).toBe("blocked")
        expect(
            screen.getByTestId("package-section-row-tenant-configuration").textContent
        ).toContain("unknown role x")
        expect(badge("trustees")).toBe("missing")
        expect(
            Array.from(
                screen.getByTestId("package-run-stages").querySelectorAll("[data-state]")
            ).map((stage) => stage.getAttribute("data-state"))
        ).toEqual(["done", "blocked", "pending", "pending"])
        fireEvent.click(screen.getByTestId("package-change-requirements"))
        const picker = screen.getByTestId("package-requirements-picker")
        expect(
            within(screen.getByTestId("package-upload-drawer")).getByTestId(
                "package-requirements-picker"
            )
        ).toBe(picker)
        expect(
            Array.from(picker.querySelectorAll("input[type=checkbox]")).map((box) =>
                box.getAttribute("aria-label")
            )
        ).toEqual([
            "tenant-configuration",
            "tenant-settings",
            "role-permissions",
            "admin-accounts",
            "trustees",
            "messaging-accounts",
            "election-event",
            "census",
        ])
        expect(mockApply).toHaveBeenCalledTimes(1)
    })

    it("keeps the tenant's applied receipt separate from the current run", async () => {
        mockHistory = [
            {
                id: "task-later-preview",
                created_at: "2026-11-26T09:41:00Z",
                execution_status: "SUCCESS",
                executed_by_user: "ops.lead",
                annotations: record("previewed"),
            },
            {
                id: "task-applied",
                created_at: "2026-11-24T14:02:00Z",
                execution_status: "SUCCESS",
                executed_by_user: "ops.lead",
                annotations: record("applied"),
            },
        ]
        await previewed()
        const receipt = screen.getByTestId("package-receipt")
        expect(receipt.textContent).toContain("task-applied")
        expect(receipt.textContent).toContain("2026-11-24T14:02:00Z")
        expect(receipt.textContent).not.toContain("task-p")
        expect(screen.getByTestId("package-run-card").textContent).toContain("task-p")
    })

    it("says no package is applied when the recent history holds none", () => {
        render(<SettingsConfigurationPackage />)
        expect(screen.getByTestId("package-receipt").textContent).toContain(
            "configurationPackage.hub.receipt.none"
        )
    })

    it("previews the same followed document again, with the new requirements, once clicked", async () => {
        await previewed()
        showAdvanced()
        fireEvent.click(screen.getByRole("checkbox", {name: "census"}))
        expect(applyButton().disabled).toBe(true)
        // The disabled Apply points at the reason it is off.
        const reason = screen.getByTestId("package-stale-reason")
        expect(applyButton().getAttribute("aria-describedby")).toBe(reason.id)
        expect(reason.textContent).toBe("configurationPackage.hub.staleReason")
        mockApply.mockResolvedValueOnce(accepted("task-p2"))
        await act(async () => {
            fireEvent.click(screen.getByTestId("package-preview-again"))
        })
        expect(mockApply.mock.calls.map(([options]) => options.variables)).toEqual([
            expect.objectContaining({documentId: "doc-1", mode: "preview"}),
            expect.objectContaining({
                documentId: "doc-1",
                mode: "preview",
                requirements: {...COMPLETE_DELIVERY, census: "optional"},
            }),
        ])
        expect(mockManagedApply).not.toHaveBeenCalled()
    })

    it("offers no Preview again when the platform read other bytes than the entered SHA-256", async () => {
        mockSha = "f".repeat(64)
        await previewed()
        expect(screen.getByTestId("package-sha-mismatch")).toBeTruthy()
        expect(screen.queryByTestId("package-preview-again")).toBeNull()
        expect(screen.getAllByTestId("package-upload")).toHaveLength(1)
    })

    it("names the previewed package and the sections that change, in order, before Apply", async () => {
        await previewed()
        fireEvent.click(applyButton())
        const identity = screen.getByTestId("package-confirm-identity")
        expect(identity.textContent).toContain("abcdef0123456789abcdef")
        expect(
            Array.from(screen.getByTestId("package-confirm-order").querySelectorAll("li")).map(
                (item) => item.textContent
            )
        ).toEqual(["configurationPackage.sections.tenant-configuration"])
        expect(mockApply.mock.calls.map(([options]) => options.variables.mode)).toEqual(["preview"])
    })

    it("shows a selected run's recorded identity and section results in History", () => {
        const applied = record("applied")
        Object.assign(applied.application, {
            managedRosterBinding: {sha256: "ab".repeat(32), generation: 3, requiredApprovals: 2},
        })
        mockHistory = [
            {
                id: "task-refused",
                created_at: "2026-11-25T10:00:00Z",
                execution_status: "FAILED",
                executed_by_user: "ops.backup",
                annotations: refusedPreview(["unknown role x"]),
            },
            {
                id: "task-applied",
                created_at: "2026-11-24T14:02:00Z",
                execution_status: "SUCCESS",
                executed_by_user: "ops.lead",
                annotations: applied,
            },
        ]
        render(<SettingsConfigurationPackage />)
        fireEvent.click(screen.getByTestId("package-rail-history"))
        expect(screen.getByTestId("package-history-detail").textContent).toContain("refused")
        fireEvent.click(screen.getByTestId("package-history-row-task-applied"))
        const detail = screen.getByTestId("package-history-detail")
        expect(detail.textContent).toContain("ops.lead")
        expect(detail.textContent).toContain("task-applied")
        expect(detail.textContent).toContain('"generation":3')
        expect(detail.querySelector('[data-badge="applied"]')).toBeTruthy()
    })

    it("lists a messaging account change in the confirmation order, as in its proposed changes", async () => {
        const view = await previewed()
        finishPortablePreview(undefined, [
            {reference: "package:ses", action: "replace", channel: "EMAIL", provider: "AWS_SES"},
        ])
        const sections = mockTask.annotations.application.sections
        sections[sections.length - 1].status.preview.needsApply = false
        view.rerender(<SettingsConfigurationPackage />)
        await act(async () => fireEvent.click(applyButton()))
        const order = Array.from(
            screen.getByTestId("package-confirm-order").querySelectorAll("li")
        ).map((item) => item.textContent)
        expect(order).toEqual([
            "configurationPackage.sections.tenant-configuration",
            "configurationPackage.sections.messaging-accounts",
        ])
        const proposed = within(screen.getByTestId("package-proposed-changes"))
        expect(proposed.getByText("configurationPackage.sections.messaging-accounts")).toBeTruthy()
        expect(mockManagedApply).not.toHaveBeenCalled()
    })

    it("makes a blocked preview historical after Change requirements and previews the same import again", async () => {
        mockApply.mockResolvedValue(accepted("task-p"))
        const {rerender} = render(<SettingsConfigurationPackage />)
        await upload()
        mockTask = task(
            "task-p",
            "FAILED",
            refusedPreview(["unknown role x"], {missingTrustees: true})
        )
        rerender(<SettingsConfigurationPackage />)
        expect(screen.queryByTestId("package-blocked-historical")).toBeNull()
        fireEvent.click(screen.getByTestId("package-change-requirements"))
        // The drawer is modal: its own controls are reachable, the page behind it is not.
        const drawer = screen.getByTestId("package-upload-drawer")
        fireEvent.click(within(drawer).getByRole("checkbox", {name: "trustees"}))
        expect(
            within(drawer).getByRole("button", {
                name: "configurationPackage.hub.actions.previewAgainSame",
            })
        ).toBeTruthy()
        expect(
            screen.queryByRole("heading", {
                name: "configurationPackage.workflow.phases.previewAgain.title",
            })
        ).toBeNull()
        // Close it as a user would, keeping the changed requirements.
        fireEvent.keyDown(drawer, {key: "Escape"})
        await waitFor(() => expect(screen.queryByTestId("package-upload-drawer")).toBeNull())
        // What that run found stays visible, as that run's history.
        expect(screen.getByTestId("package-blocked").getAttribute("data-historical")).toBe("true")
        expect(screen.getByTestId("package-blocked").textContent).toContain("unknown role x")
        expect(screen.getByTestId("package-sections-historical")).toBeTruthy()
        expect(
            within(screen.getByTestId("package-workflow")).getByRole("heading", {
                name: "configurationPackage.workflow.phases.previewAgain.title",
            })
        ).toBeTruthy()
        // Fail closed: nothing to apply until a new Preview succeeds.
        expect(applyButton().disabled).toBe(true)
        mockApply.mockResolvedValueOnce(accepted("task-p2"))
        // Preview again from the page header: the same import, no new upload.
        await act(async () => {
            fireEvent.click(
                within(screen.getByTestId("package-workflow")).getByRole("button", {
                    name: "configurationPackage.hub.actions.previewAgain",
                })
            )
        })
        expect(screen.queryByTestId("package-upload-drawer")).toBeNull()
        expect(mockApply.mock.calls.map(([options]) => options.variables)).toEqual([
            expect.objectContaining({documentId: "doc-1", mode: "preview"}),
            expect.objectContaining({
                documentId: "doc-1",
                mode: "preview",
                requirements: {
                    sections: COMPLETE_DELIVERY.sections.filter((s) => s !== "trustees"),
                    census: "required",
                },
            }),
        ])
        expect(applyButton().disabled).toBe(true)
        // The new Preview's own refusal is current, not historical.
        mockTask = task("task-p2", "FAILED", refusedPreview(["unknown role x"]))
        rerender(<SettingsConfigurationPackage />)
        expect(screen.getByTestId("package-blocked").getAttribute("data-historical")).toBeNull()
        expect(mockManagedApply).not.toHaveBeenCalled()
    })

    it("previews the same import again from inside the requirements drawer", async () => {
        mockApply.mockResolvedValue(accepted("task-p"))
        const {rerender} = render(<SettingsConfigurationPackage />)
        await upload()
        mockTask = task(
            "task-p",
            "FAILED",
            refusedPreview(["unknown role x"], {missingTrustees: true})
        )
        rerender(<SettingsConfigurationPackage />)
        fireEvent.click(screen.getByTestId("package-change-requirements"))
        const drawer = screen.getByTestId("package-upload-drawer")
        // Not offered until the requirements differ from the refused run's.
        expect(
            within(drawer).queryByRole("button", {
                name: "configurationPackage.hub.actions.previewAgainSame",
            })
        ).toBeNull()
        fireEvent.click(within(drawer).getByRole("checkbox", {name: "trustees"}))
        mockApply.mockResolvedValueOnce(accepted("task-p2"))
        await act(async () => {
            fireEvent.click(
                within(drawer).getByRole("button", {
                    name: "configurationPackage.hub.actions.previewAgainSame",
                })
            )
        })
        await waitFor(() => expect(screen.queryByTestId("package-upload-drawer")).toBeNull())
        expect(mockApply.mock.calls.map(([options]) => options.variables)).toEqual([
            expect.objectContaining({documentId: "doc-1", mode: "preview"}),
            expect.objectContaining({
                documentId: "doc-1",
                mode: "preview",
                requirements: {
                    sections: COMPLETE_DELIVERY.sections.filter((s) => s !== "trustees"),
                    census: "required",
                },
            }),
        ])
        expect(applyButton().disabled).toBe(true)
        expect(mockManagedApply).not.toHaveBeenCalled()
    })

    it("routes a missing managed signer roster to Signing trust, keeping import disabled", () => {
        mockRoster = {...managedRoster, generation: "0", active_sha256: null}
        render(<SettingsConfigurationPackage />)
        expect((screen.getByTestId("package-upload") as HTMLButtonElement).disabled).toBe(true)
        expect(screen.queryByTestId("package-check-trust-again")).toBeNull()
        fireEvent.click(screen.getByTestId("package-open-trust"))
        expect(screen.getByTestId("package-trust-panel")).toBeTruthy()
        expect(screen.getByTestId("package-rail-trust").getAttribute("aria-current")).toBe("page")
        expect(mockApply).not.toHaveBeenCalled()
    })

    it("offers Check again when the trust check failed, re-reading only", async () => {
        mockRoster = null
        render(<SettingsConfigurationPackage />)
        expect((screen.getByTestId("package-upload") as HTMLButtonElement).disabled).toBe(true)
        expect(screen.queryByTestId("package-open-trust")).toBeNull()
        await act(async () => {
            fireEvent.click(screen.getByTestId("package-check-trust-again"))
        })
        expect(mockRosterRead).toHaveBeenCalledTimes(1)
        expect(mockApply).not.toHaveBeenCalled()
    })

    it("never shows complete for an applied record on a task that did not succeed", async () => {
        const {rerender} = await previewed()
        mockApply.mockResolvedValueOnce(accepted("task-a"))
        await apply()
        mockTask = task("task-a", "FAILED", record("applied"))
        rerender(<SettingsConfigurationPackage />)
        expect(screen.getByTestId("package-workflow").getAttribute("data-phase")).toBe("attention")
        expect(screen.getByTestId("package-outcome").textContent).toBe(
            "configurationPackage.hub.appliedNotConfirmed"
        )
        expect(
            screen
                .getByTestId("package-section-row-tenant-configuration")
                .querySelector("[data-badge]")
                ?.getAttribute("data-badge")
        ).toBe("notConfirmed")
        mockTask = task("task-a", "SUCCESS", record("applied"))
        rerender(<SettingsConfigurationPackage />)
        expect(screen.getByTestId("package-workflow").getAttribute("data-phase")).toBe("complete")
    })

    it("shows an applied record on a task that did not succeed as not confirmed in History", () => {
        mockHistory = [
            {
                id: "task-contradicted",
                created_at: "2026-11-25T10:00:00Z",
                execution_status: "FAILED",
                executed_by_user: "ops.backup",
                annotations: record("applied"),
            },
            {
                id: "task-confirmed",
                created_at: "2026-11-24T14:02:00Z",
                execution_status: "SUCCESS",
                executed_by_user: "ops.lead",
                annotations: record("applied"),
            },
        ]
        render(<SettingsConfigurationPackage />)
        // The receipt skips the contradicted run.
        expect(screen.getByTestId("package-receipt").textContent).toContain("task-confirmed")
        expect(screen.getByTestId("package-receipt").textContent).not.toContain("task-contradicted")
        fireEvent.click(screen.getByTestId("package-rail-history"))
        const chip = (id: string) =>
            screen.getByTestId(`package-history-row-${id}`).querySelector(".MuiChip-root")
        expect(chip("task-contradicted")?.textContent).toBe(
            "configurationPackage.hub.appliedNotConfirmed"
        )
        expect(chip("task-contradicted")?.className).not.toContain("MuiChip-colorSuccess")
        expect(chip("task-confirmed")?.textContent).toBe("applied")
        expect(chip("task-confirmed")?.className).toContain("MuiChip-colorSuccess")
        const detail = screen.getByTestId("package-history-detail")
        expect(detail.textContent).toContain("configurationPackage.hub.appliedNotConfirmed")
        expect(detail.querySelector('[data-badge="applied"]')).toBeNull()
        expect(detail.querySelector('[data-badge="notConfirmed"]')).toBeTruthy()
        fireEvent.click(screen.getByTestId("package-history-row-task-confirmed"))
        expect(
            screen.getByTestId("package-history-detail").querySelector('[data-badge="applied"]')
        ).toBeTruthy()
    })
})
