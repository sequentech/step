// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {IvrExpectedInput} from "./IvrCall"

/** What a keypad hint's wording fills in; a plain record, as i18n libraries take. */
export type KeypadHintValues = {
    maxDigits: number
    /** The keys the prompt accepts as the Lambda sent them ("2,1"), or `""` for any. */
    validInputs: string
    /** The same keys as somebody would say them: "1 or 2", "0, 1 or 2". */
    keys: string
    timeout: number
}

/** A keypad hint's two wordings, in whatever words and language the host uses. */
export interface KeypadHintWording {
    /** The prompt names the keys it accepts. */
    listed: (values: KeypadHintValues) => string
    /** Any digits will do, up to `maxDigits`, such as for a PIN. */
    anyKeys: (values: KeypadHintValues) => string
    /** The word before the last of the keys: "1, 2 or 3". English when not given. */
    or?: string
}

/** Digits by their value (`0`, `1`, `01`, `10`), then the keypad's symbols (`*`, `#`). */
const byKey = (a: string, b: string): number => {
    const digits = /^\d+$/
    if (digits.test(a) !== digits.test(b)) return digits.test(a) ? -1 : 1
    return digits.test(a) ? Number(a) - Number(b) || a.localeCompare(b) : a.localeCompare(b)
}

/**
 * The keys a prompt accepts, as somebody would say them: "0, 1 or 2".
 *
 * The Lambda sends them comma-separated in no particular order ("2,1" for a language
 * menu), each padded to the prompt's length ("01,00,03").
 */
export const keyList = (validInputs: string, or: string): string => {
    const keys = Array.from(
        new Set(
            validInputs
                .split(",")
                .map((key) => key.trim())
                .filter(Boolean)
        )
    ).sort(byKey)
    const last = keys.pop()
    if (last === undefined) return ""
    return keys.length ? `${keys.join(", ")} ${or} ${last}` : last
}

/**
 * What the keypad says while the call waits for keys.
 *
 * The Lambda lists the keys a prompt accepts, or none when any digits will do
 * (`DtmfValidInputs::Anything`, a PIN up to `max_digits`): filling that empty list
 * into "valid inputs={{validInputs}}" would leave "valid inputs=" and nothing.
 */
export function keypadHint(
    expected: Pick<IvrExpectedInput, "valid_inputs" | "max_digits" | "timeout">,
    wording: KeypadHintWording
): string {
    const validInputs = expected.valid_inputs.trim()
    const keys = keyList(validInputs, wording.or ?? "or")
    const values = {maxDigits: expected.max_digits, validInputs, keys, timeout: expected.timeout}
    return keys ? wording.listed(values) : wording.anyKeys(values)
}
