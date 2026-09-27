// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {keypadHint, type KeypadHintWording} from "./keypadHint"

const say: KeypadHintWording = {
    listed: ({maxDigits, validInputs, timeout}) =>
        `Up to ${maxDigits} of ${validInputs}, within ${timeout}s`,
    anyKeys: ({maxDigits, timeout}) => `Up to ${maxDigits} digits, within ${timeout}s`,
}

describe("keypadHint", () => {
    it("names the keys a prompt accepts", () => {
        expect(keypadHint({valid_inputs: "1,2,0", max_digits: 1, timeout: 5}, say)).toBe(
            "Up to 1 of 1,2,0, within 5s"
        )
    })

    it("trims the keys it names", () => {
        expect(keypadHint({valid_inputs: " 0-9 ", max_digits: 3, timeout: 10}, say)).toBe(
            "Up to 3 of 0-9, within 10s"
        )
    })

    it("says any digits will do when the Lambda lists no keys", () => {
        // `DtmfValidInputs::Anything` goes down the wire as an empty list (a PIN).
        for (const valid_inputs of ["", "  "]) {
            expect(keypadHint({valid_inputs, max_digits: 8, timeout: 5}, say)).toBe(
                "Up to 8 digits, within 5s"
            )
        }
    })

    it("hands each wording the values it fills in", () => {
        const listed = jest.fn(() => "listed")
        const anyKeys = jest.fn(() => "any")
        keypadHint({valid_inputs: "", max_digits: 4, timeout: 7}, {listed, anyKeys})
        expect(anyKeys).toHaveBeenCalledWith({maxDigits: 4, validInputs: "", timeout: 7})
        expect(listed).not.toHaveBeenCalled()
    })
})
