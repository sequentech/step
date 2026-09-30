// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {IMonitoringEditorApi} from "./api"
import {EDuplicateResult, duplicateWidgetOnDashboard} from "./duplicateWidget"
import {
    EMonitoringConfigKind,
    EMonitoringProblemSeverity,
    EMonitoringSaveChange,
    EMonitoringSaveStatus,
    type IMonitoringConfigDocument,
    type IMonitoringSaveRequest,
    type TMonitoringSaveOutcome,
} from "./types"

const DASHBOARD = [
    "id: overview",
    "title: Overview",
    "layout:",
    "  - widget: turnout",
    "    width: 6",
    "  - widget: activity",
    "    width: 6",
    "",
].join("\n")

const WIDGET = "id: activity\ntitle: Activity\n"

const copyTitle = (title: string) => `${title} (copy)`

const document = (kind: EMonitoringConfigKind, key: string, yaml: string | null, revision = 4) =>
    ({kind, key, yaml, revision}) satisfies IMonitoringConfigDocument

const saved: TMonitoringSaveOutcome = {
    status: EMonitoringSaveStatus.SAVED,
    revision: 1,
    generation: 2,
    warnings: [],
}

const fakeApi = (
    save: (request: IMonitoringSaveRequest) => TMonitoringSaveOutcome = () => saved,
    dashboardYaml: string | null = DASHBOARD
) => {
    const api = {
        getConfig: jest.fn(async ({kind, key}: {kind: EMonitoringConfigKind; key: string}) =>
            kind === EMonitoringConfigKind.DASHBOARD
                ? document(kind, key, dashboardYaml, 9)
                : document(kind, key, WIDGET)
        ),
        listConfig: jest.fn(async () => [
            {kind: EMonitoringConfigKind.WIDGET, key: "activity", revision: 4},
            {kind: EMonitoringConfigKind.WIDGET, key: "activity-copy", revision: 1},
            {kind: EMonitoringConfigKind.DASHBOARD, key: "activity-copy-2", revision: 1},
        ]),
        saveConfig: jest.fn(async (request: IMonitoringSaveRequest) => save(request)),
    }
    return api as unknown as IMonitoringEditorApi & typeof api
}

describe("duplicateWidgetOnDashboard", () => {
    it("titles the copy apart from the original, and leaves an untitled widget untitled", async () => {
        const api = fakeApi()
        api.getConfig.mockImplementation(async ({kind, key}) =>
            kind === EMonitoringConfigKind.DASHBOARD
                ? document(kind, key, DASHBOARD, 9)
                : document(kind, key, "id: activity\nsource: voter_turnout\n")
        )
        await duplicateWidgetOnDashboard(api, {
            dashboardId: "overview",
            widgetId: "activity",
            copyTitle,
        })
        expect(api.saveConfig.mock.calls[0][0].yaml).toBe(
            "id: activity-copy-2\nsource: voter_turnout\n"
        )
    })

    it("saves a copy under a free id and places it after the original", async () => {
        const api = fakeApi()
        const outcome = await duplicateWidgetOnDashboard(api, {
            dashboardId: "overview",
            widgetId: "activity",
            copyTitle,
        })
        expect(outcome).toEqual({result: EDuplicateResult.DONE, id: "activity-copy-2"})
        const [copy, dashboard] = api.saveConfig.mock.calls.map(([request]) => request)
        expect(copy).toEqual({
            kind: EMonitoringConfigKind.WIDGET,
            key: "activity-copy-2",
            yaml: "id: activity-copy-2\ntitle: Activity (copy)\n",
            change: EMonitoringSaveChange.UPSERT,
        })
        expect(dashboard).toEqual(
            expect.objectContaining({
                kind: EMonitoringConfigKind.DASHBOARD,
                key: "overview",
                expected_revision: 9,
            })
        )
        expect(dashboard.yaml).toContain(
            "  - widget: activity\n    width: 6\n  - widget: activity-copy-2\n    width: 6\n"
        )
    })

    it("stops before placing anything when the copy is refused", async () => {
        const problem = {
            severity: EMonitoringProblemSeverity.ERROR,
            code: "invalid_id",
            path: "id",
            message: "Too long",
        }
        const api = fakeApi(() => ({status: EMonitoringSaveStatus.INVALID, problems: [problem]}))
        const outcome = await duplicateWidgetOnDashboard(api, {
            dashboardId: "overview",
            widgetId: "activity",
            copyTitle,
        })
        expect(outcome).toEqual({result: EDuplicateResult.NOT_SAVED, problem: "Too long"})
        expect(api.saveConfig).toHaveBeenCalledTimes(1)
    })

    it("says the copy exists but is not placed when the dashboard save is refused", async () => {
        const api = fakeApi((request) =>
            request.kind === EMonitoringConfigKind.DASHBOARD
                ? {status: EMonitoringSaveStatus.CONFLICT, current_revision: 10}
                : saved
        )
        const outcome = await duplicateWidgetOnDashboard(api, {
            dashboardId: "overview",
            widgetId: "activity",
            copyTitle,
        })
        expect(outcome).toEqual({
            result: EDuplicateResult.NOT_PLACED,
            id: "activity-copy-2",
            status: EMonitoringSaveStatus.CONFLICT,
        })
    })

    it("refuses a widget that is not on the dashboard, before saving anything", async () => {
        const api = fakeApi()
        await expect(
            duplicateWidgetOnDashboard(api, {
                dashboardId: "overview",
                widgetId: "missing",
                copyTitle,
            })
        ).rejects.toThrow("missing")
        expect(api.saveConfig).not.toHaveBeenCalled()
    })
})
