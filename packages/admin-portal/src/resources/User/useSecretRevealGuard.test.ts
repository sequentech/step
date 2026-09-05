/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {renderHook} from "@testing-library/react"
import {useSecretRevealGuard} from "./useSecretRevealGuard"

it("accepts only a response for the currently authorized editor", () => {
    const {result, rerender, unmount} = renderHook(
        ({context, allowed}) => useSecretRevealGuard(context, allowed),
        {initialProps: {context: "tenant:event:voter-a", allowed: true}}
    )
    const first = result.current()
    expect(first()).toBe(true)
    rerender({context: "tenant:event:voter-b", allowed: true})
    expect(first()).toBe(false)
    const second = result.current()
    rerender({context: "tenant:event:voter-b", allowed: false})
    expect(second()).toBe(false)
    expect(result.current()()).toBe(false)
    rerender({context: "tenant:event:voter-a", allowed: true})
    expect(first()).toBe(false)
    const last = result.current()
    unmount()
    expect(last()).toBe(false)
})
