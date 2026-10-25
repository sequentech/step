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
const mockStopPolling = jest.fn()

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
                        : {sequent_backend_tasks_execution: [mockTask]},
                startPolling: jest.fn(),
                stopPolling: mockStopPolling,
            }
        }
        return {data: {sequent_backend_tasks_execution: []}}
    },
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
    }),
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["t1"]}))
jest.mock("@/providers/AuthContextProvider", () => {
    const React = require("react")
    return {AuthContext: React.createContext({isAuthorized: () => mockAllowed})}
})
jest.mock("@/components/election-event/import-data/ImportDataDrawer", () => ({
    ImportDataDrawer: ({open, doImport}: any) =>
        open ? (
            <button onClick={() => doImport("doc-1", "sha")} data-testid="drawer-upload">
                upload
            </button>
        ) : null,
}))

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
})

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
    mockTask = task("task-a", "IN_PROGRESS", null)
    await act(async () => {
        fireEvent.click(applyButton())
    })
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

    mockTask = task("task-x", "SUCCESS", record("refused"))
    rerender(<SettingsConfigurationPackage />)
    expect(screen.getByTestId("package-outcome").textContent).toBe(
        "configurationPackage.outcome.refused"
    )
    expect(screen.getByTestId("section-tenant-configuration").textContent).toContain(
        'configurationPackage.earlierApplied {"tasks":"task-earlier"}'
    )
    expect(applyButton().disabled).toBe(true)
})

it("never resubmits on its own after a failed submission or an unknown outcome", async () => {
    mockApply.mockRejectedValueOnce(new Error("network down"))
    const {rerender} = render(<SettingsConfigurationPackage />)
    await upload()
    expect(screen.getByTestId("package-error").textContent).toBe("network down")
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
