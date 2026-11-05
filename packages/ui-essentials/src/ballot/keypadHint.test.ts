// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {keyList, keypadHint, type KeypadHintWording} from "./keypadHint"

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
        expect(anyKeys).toHaveBeenCalledWith({
            maxDigits: 4,
            validInputs: "",
            keys: "",
            timeout: 7,
        })
        expect(listed).not.toHaveBeenCalled()
    })
})

describe("the keys a prompt names, as a sentence", () => {
    const press: KeypadHintWording = {
        listed: ({keys, timeout}) => `Press ${keys}, within ${timeout}s`,
        anyKeys: ({maxDigits, timeout}) => `Up to ${maxDigits} digits, within ${timeout}s`,
    }

    it("orders them and joins the last with 'or'", () => {
        // The Election Architect's language menu said "Up to 1 of 2,1, within 5s".
        expect(keypadHint({valid_inputs: "2,1", max_digits: 1, timeout: 5}, press)).toBe(
            "Press 1 or 2, within 5s"
        )
        expect(keypadHint({valid_inputs: "1,2,0", max_digits: 1, timeout: 5}, press)).toBe(
            "Press 0, 1 or 2, within 5s"
        )
        expect(keypadHint({valid_inputs: "1", max_digits: 1, timeout: 10}, press)).toBe(
            "Press 1, within 10s"
        )
    })

    it("puts the symbols after the digits and names each key once", () => {
        expect(keyList("#, 2,1,2", "or")).toBe("1, 2 or #")
        // The Lambda pads a longer menu's numbers to one length: "01,00,03".
        expect(keyList("03,01,10", "or")).toBe("01, 03 or 10")
        expect(keyList(" , ", "or")).toBe("")
    })

    it("says 'or' in the host's language", () => {
        expect(
            keypadHint({valid_inputs: "2,1", max_digits: 1, timeout: 5}, {...press, or: "o"})
        ).toBe("Press 1 o 2, within 5s")
    })

    it("still hands over the keys as the Lambda sent them", () => {
        const listed = jest.fn(() => "listed")
        keypadHint({valid_inputs: " 2,1 ", max_digits: 1, timeout: 5}, {...say, listed})
        expect(listed).toHaveBeenCalledWith({
            maxDigits: 1,
            validInputs: "2,1",
            keys: "1 or 2",
            timeout: 5,
        })
    })
})
