// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {generateRandomPassword} from "./Password"

afterEach(() => {
    jest.restoreAllMocks()
})
it("maps secure random bytes to the documented alphabet including punctuation", () => {
    const random = jest.spyOn(crypto, "getRandomValues").mockImplementation((array) => {
        ;(array as Uint8Array).set([0, 25, 26, 51, 52, 61, 62, 63, 64, 65, 129, 255])
        return array
    })
    expect(generateRandomPassword()).toBe("azAZ09-_.a.8")
    expect(random).toHaveBeenCalledTimes(1)
    expect(random.mock.calls[0][0]).toBeInstanceOf(Uint8Array)
})
it("honors requested lengths without changing the alphabet", () => {
    expect(generateRandomPassword(0)).toBe("")
    expect(generateRandomPassword(40)).toMatch(/^[a-zA-Z0-9_.-]{40}$/)
    expect(() => generateRandomPassword(-1)).toThrow(expect.objectContaining({name: "RangeError"}))
})
