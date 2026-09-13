/**
 * @jest-environment jsdom
 * @jest-environment-options {"url":"http://localhost/"}
 */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {expect, it, jest} from "@jest/globals"
import {setCookie} from "./cookies"

it("supports local HTTP development without an invalid domain attribute", () => {
    const setter = jest.spyOn(Document.prototype, "cookie", "set")
    try {
        setCookie("language", "en")
        expect(setter).toHaveBeenCalledWith("language=en; Path=/; SameSite=Lax")
    } finally {
        setter.mockRestore()
    }
})
