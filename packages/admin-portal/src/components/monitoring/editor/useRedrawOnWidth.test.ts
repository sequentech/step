/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {renderHook} from "@testing-library/react"
import type {YamlDraftController} from "./yamlDraft"
import {useRedrawOnWidth} from "./useRedrawOnWidth"

const fakeController = () =>
    ({previewNow: jest.fn(async () => undefined)}) as unknown as YamlDraftController & {
        previewNow: jest.Mock
    }

describe("useRedrawOnWidth", () => {
    it("leaves the first preview to the document's load", () => {
        const controller = fakeController()
        renderHook(() => useRedrawOnWidth(controller, 360, true))
        expect(controller.previewNow).not.toHaveBeenCalled()
    })

    it("draws the preview again once the pane it shows in is measured", () => {
        const controller = fakeController()
        const hook = renderHook(({width}) => useRedrawOnWidth(controller, width, true), {
            initialProps: {width: 360},
        })
        hook.rerender({width: 600})
        expect(controller.previewNow).toHaveBeenCalledTimes(1)
        hook.rerender({width: 600})
        expect(controller.previewNow).toHaveBeenCalledTimes(1)
    })

    it("does not draw a document still loading, whose own render comes at the new width", () => {
        const controller = fakeController()
        const hook = renderHook(({width, ready}) => useRedrawOnWidth(controller, width, ready), {
            initialProps: {width: 360, ready: false},
        })
        hook.rerender({width: 600, ready: false})
        hook.rerender({width: 600, ready: true})
        expect(controller.previewNow).not.toHaveBeenCalled()
    })
})
