/**
 * @jest-environment jsdom
 * @jest-environment-options {"url":"https://voting.example.org/"}
 */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {afterEach, expect, it, jest} from "@jest/globals"
import {getValueFromCookie, setCookie} from "./cookies"

afterEach(() => {
    jest.restoreAllMocks()
})

it("round-trips cookie names and values that need encoding", () => {
    setCookie("voter language", "français = yes; café")
    expect(getValueFromCookie("voter language")).toBe("français = yes; café")
})

it("retains equals signs and accepts optional whitespace between cookies", () => {
    jest.spyOn(Document.prototype, "cookie", "get").mockReturnValue(
        "first=one;opaque=token==; empty=; last=done"
    )
    expect(getValueFromCookie("opaque")).toBe("token==")
    expect(getValueFromCookie("last")).toBe("done")
    expect(getValueFromCookie("empty")).toBeUndefined()
    expect(getValueFromCookie("missing")).toBeUndefined()
})

it("keeps a malformed percent escape readable instead of throwing", () => {
    jest.spyOn(Document.prototype, "cookie", "get").mockReturnValue("legacy=%ZZ")
    expect(getValueFromCookie("legacy")).toBe("%ZZ")
})

it("sets the secure parent-domain cookie with explicit path and same-site policy", () => {
    const setter = jest.spyOn(Document.prototype, "cookie", "set")
    setCookie("language", "en")
    expect(setter).toHaveBeenCalledWith(
        "language=en; Path=/; SameSite=Lax; Domain=example.org; Secure"
    )
})
