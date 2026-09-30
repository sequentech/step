/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {act, renderHook, waitFor} from "@testing-library/react"
import type {IMonitoringEditorApi} from "./api"
import {
    EMonitoringConfigKind,
    EMonitoringProblemSeverity,
    EMonitoringSaveStatus,
    EMonitoringValidationResult,
    type IMonitoringConfigDocument,
    type TMonitoringSaveOutcome,
} from "./types"
import {YamlDraftController} from "./yamlDraft"
import {
    authorName,
    EMessageTone,
    useConfigDocument,
    type IDocumentMessages,
} from "./useConfigDocument"

// A new `t` on every render, as i18next hands out when the language changes.
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
    }),
}))

const MESSAGES: IDocumentMessages = {
    loadFailed: "loadFailed",
    saved: "saved",
    refused: "refused",
    validated: "validated",
    invalid: "invalid",
    requestFailed: "requestFailed",
}

const TEXT = "id: turnout\ntitle: Turnout\n"

const stored = (revision: number, yaml: string | null = TEXT): IMonitoringConfigDocument => ({
    kind: EMonitoringConfigKind.WIDGET,
    key: "turnout",
    yaml,
    revision,
    author: {id: "u-ana", name: "Ana"},
    created_at: "2026-09-29T10:00:00Z",
})

const deferred = <T>() => {
    let resolve: (value: T) => void = () => undefined
    const promise = new Promise<T>((done) => {
        resolve = done
    })
    return {promise, resolve}
}

const fakeApi = (overrides: Partial<IMonitoringEditorApi> = {}): IMonitoringEditorApi => ({
    validateConfig: jest.fn(async () => ({
        result: EMonitoringValidationResult.VALID,
        problems: [],
    })),
    saveConfig: jest.fn(
        async (): Promise<TMonitoringSaveOutcome> => ({
            status: EMonitoringSaveStatus.SAVED,
            revision: 8,
            generation: 2,
            warnings: [],
        })
    ),
    renderWidget: jest.fn(),
    getConfig: jest.fn(async () => stored(7)),
    listConfig: jest.fn(async () => []),
    listPresets: jest.fn(async () => []),
    resetToPreset: jest.fn(async () => ({generation: 1})),
    ...overrides,
})

const open = async (api: IMonitoringEditorApi) => {
    const controller = new YamlDraftController({text: ""})
    const hook = renderHook(() =>
        useConfigDocument({
            api,
            kind: EMonitoringConfigKind.WIDGET,
            key: "turnout",
            open: true,
            controller,
            messages: MESSAGES,
        })
    )
    await waitFor(() => expect(controller.getState().text).toBe(TEXT))
    return {controller, hook}
}

describe("useConfigDocument", () => {
    it("loads once, however often the translation function changes", async () => {
        const api = fakeApi()
        const {hook} = await open(api)
        hook.rerender()
        hook.rerender()
        expect(api.getConfig).toHaveBeenCalledTimes(1)
    })

    it("a save marks the text it sent as saved, not what was typed meanwhile", async () => {
        const reply = deferred<TMonitoringSaveOutcome>()
        const api = fakeApi({saveConfig: jest.fn(() => reply.promise)})
        const {controller, hook} = await open(api)
        const sent = `${TEXT}height: 3\n`
        controller.setText(sent)
        let saving: Promise<boolean> = Promise.resolve(false)
        act(() => {
            saving = hook.result.current.save()
        })
        controller.setText(`${TEXT}height: 30\n`)
        await act(async () => {
            reply.resolve({
                status: EMonitoringSaveStatus.SAVED,
                revision: 8,
                generation: 2,
                warnings: [],
            })
            await saving
        })
        expect(api.saveConfig).toHaveBeenCalledWith(expect.objectContaining({yaml: sent}))
        expect(controller.getState()).toEqual(
            expect.objectContaining({baseline: sent, dirty: true})
        )
    })

    it("after a save, names the author and time Harvest stored the revision with", async () => {
        const api = fakeApi({
            saveConfig: jest.fn(async () => ({
                status: EMonitoringSaveStatus.SAVED as const,
                revision: 8,
                generation: 2,
                warnings: [],
                author: {id: "u-admin", name: "Admin admin"},
                created_at: "2026-09-30T08:00:00Z",
            })),
        })
        const {controller, hook} = await open(api)
        controller.setText(`${TEXT}height: 3\n`)
        await act(async () => {
            await hook.result.current.save()
        })
        expect(hook.result.current.revision).toEqual({
            revision: 8,
            author: {id: "u-admin", name: "Admin admin"},
            createdAt: "2026-09-30T08:00:00Z",
        })
        expect(authorName(hook.result.current.revision?.author)).toBe("Admin admin")
    })

    it("after a save to an older Harvest, which names no author, still dates it", async () => {
        const {controller, hook} = await open(fakeApi())
        controller.setText(`${TEXT}height: 3\n`)
        await act(async () => {
            await hook.result.current.save()
        })
        expect(hook.result.current.revision?.revision).toBe(8)
        expect(hook.result.current.revision?.author).toBeUndefined()
        expect(hook.result.current.revision?.createdAt).toEqual(expect.any(String))
    })

    it("shows the warnings a save let through", async () => {
        const warning = {
            severity: EMonitoringProblemSeverity.WARNING,
            code: "chart_schema",
            path: "title",
            message: "Long title",
        }
        const api = fakeApi({
            saveConfig: jest.fn(async () => ({
                status: EMonitoringSaveStatus.SAVED as const,
                revision: 8,
                generation: 2,
                warnings: [warning],
            })),
        })
        const {controller, hook} = await open(api)
        controller.setText(`${TEXT}height: 3\n`)
        await act(async () => {
            await hook.result.current.save()
        })
        expect(hook.result.current.message?.tone).toBe(EMessageTone.WARNING)
        expect(controller.getState().serverProblems).toEqual([expect.objectContaining(warning)])
    })

    it("after a conflict, keeping the draft replaces the revision fetched, not the one reported", async () => {
        const saveConfig = jest
            .fn()
            .mockResolvedValueOnce({
                status: EMonitoringSaveStatus.CONFLICT,
                current_revision: null,
                author: null,
                time: null,
            })
            .mockResolvedValueOnce({
                status: EMonitoringSaveStatus.SAVED,
                revision: 13,
                generation: 3,
                warnings: [],
            })
        const getConfig = jest
            .fn()
            .mockResolvedValueOnce(stored(7))
            .mockResolvedValueOnce(stored(12, "id: turnout\ntitle: Theirs\n"))
        const api = fakeApi({saveConfig, getConfig})
        const {controller, hook} = await open(api)
        controller.setText(`${TEXT}height: 3\n`)
        await act(async () => {
            await hook.result.current.save()
        })
        expect(hook.result.current.conflict?.currentRevision).toBeNull()
        await waitFor(() =>
            expect(hook.result.current.conflict?.theirs).toBe("id: turnout\ntitle: Theirs\n")
        )
        act(() => hook.result.current.keepEditing())
        await act(async () => {
            await hook.result.current.save()
        })
        expect(saveConfig).toHaveBeenLastCalledWith(
            expect.objectContaining({expected_revision: 12})
        )
    })

    it("when the saved revision cannot be fetched, says so and leaves Reload to try again", async () => {
        const getConfig = jest
            .fn()
            .mockResolvedValueOnce(stored(7))
            .mockRejectedValueOnce(new Error("offline"))
        const api = fakeApi({
            getConfig,
            saveConfig: jest.fn(async () => ({
                status: EMonitoringSaveStatus.CONFLICT as const,
                current_revision: 12,
            })),
        })
        const {controller, hook} = await open(api)
        controller.setText(`${TEXT}height: 3\n`)
        await act(async () => {
            await hook.result.current.save()
        })
        await waitFor(() => expect(hook.result.current.conflict?.theirsError).toContain("offline"))
        expect(hook.result.current.conflict?.theirs).toBeUndefined()
    })

    it("explains a refusal Harvest gave a code for", async () => {
        const api = fakeApi({
            saveConfig: jest.fn(async () => {
                throw {graphQLErrors: [{message: "busy", extensions: {code: "MONITORING_BUSY"}}]}
            }),
        })
        const {controller, hook} = await open(api)
        controller.setText(`${TEXT}height: 3\n`)
        await act(async () => {
            await hook.result.current.save()
        })
        expect(hook.result.current.message).toEqual({
            tone: EMessageTone.ERROR,
            text: "monitoring.editor.errors.busy",
        })
    })

    it("applies Validate's answer only to the text it checked", async () => {
        const reply = deferred<Awaited<ReturnType<IMonitoringEditorApi["validateConfig"]>>>()
        const api = fakeApi({validateConfig: jest.fn(() => reply.promise)})
        const {controller, hook} = await open(api)
        let validating: Promise<void> = Promise.resolve()
        act(() => {
            validating = hook.result.current.validate()
        })
        controller.setText(`${TEXT}height: 3\n`)
        await act(async () => {
            reply.resolve({
                result: EMonitoringValidationResult.INVALID,
                problems: [
                    {
                        severity: EMonitoringProblemSeverity.ERROR,
                        code: "x",
                        path: "",
                        message: "old",
                    },
                ],
            })
            await validating
        })
        expect(controller.getState().serverProblems).toEqual([])
    })
})
