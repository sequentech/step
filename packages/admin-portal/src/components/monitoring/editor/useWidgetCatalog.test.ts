/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {renderHook, waitFor} from "@testing-library/react"
import type {IMonitoringEditorApi} from "./api"
import {EMonitoringConfigKind} from "./types"
import {useWidgetCatalog} from "./useWidgetCatalog"

// A new `t` on every render, as i18next hands out when the language changes.
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
    }),
}))

const fakeApi = () =>
    ({
        listConfig: jest.fn(async () => [
            {kind: EMonitoringConfigKind.WIDGET, key: "turnout", revision: 1},
            {kind: EMonitoringConfigKind.WIDGET, key: "activity", revision: 1},
            {kind: EMonitoringConfigKind.THEME, key: "default", revision: 1},
        ]),
        getConfig: jest.fn(async ({kind, key}: {kind: EMonitoringConfigKind; key: string}) => ({
            kind,
            key,
            yaml: `id: ${key}\ntitle: ${key}\nsource: voter_turnout\n`,
            revision: 1,
        })),
    }) as unknown as IMonitoringEditorApi & {
        listConfig: jest.Mock
        getConfig: jest.Mock
    }

describe("useWidgetCatalog", () => {
    it("reads each widget once per opening, however often the translation function changes", async () => {
        const api = fakeApi()
        const hook = renderHook(({open}) => useWidgetCatalog(api, open), {
            initialProps: {open: true},
        })
        await waitFor(() => expect(hook.result.current.catalog).toHaveLength(2))
        hook.rerender({open: true})
        hook.rerender({open: true})
        await waitFor(() => expect(hook.result.current.themes).toEqual(["default"]))
        expect(api.getConfig).toHaveBeenCalledTimes(2)
        expect(api.getConfig).toHaveBeenCalledWith({
            kind: EMonitoringConfigKind.WIDGET,
            key: "turnout",
        })
    })

    it("reads nothing while closed", () => {
        const api = fakeApi()
        renderHook(() => useWidgetCatalog(api, false))
        expect(api.listConfig).not.toHaveBeenCalled()
    })
})
