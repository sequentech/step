/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {createRef, type ForwardedRef} from "react"
import {expect, it, jest, afterEach} from "@jest/globals"
import {cleanup, render} from "@testing-library/react"
import {useForwardedRef} from "./ref"

afterEach(cleanup)

function Input({forwardedRef}: {forwardedRef: ForwardedRef<HTMLInputElement>}) {
    return <input aria-label="Write-in name" ref={useForwardedRef(forwardedRef)} />
}

it("exposes the mounted input through object and callback refs", () => {
    const objectRef = createRef<HTMLInputElement>()
    const callback = jest.fn<(element: HTMLInputElement | null) => void>()
    const view = render(<Input forwardedRef={objectRef} />)
    const input = view.getByRole("textbox")
    if (!(input instanceof HTMLInputElement)) throw new Error("fixture did not render an input")
    expect(objectRef.current).toBe(input)
    view.rerender(<Input forwardedRef={callback} />)
    expect(callback.mock.calls).toHaveLength(1)
    expect(callback.mock.calls[0][0]).toBe(input)
    // A ref is optional: omitting it must not prevent the real input mounting.
    view.rerender(<Input forwardedRef={null} />)
    expect(view.getByRole("textbox")).toBe(input)
})
