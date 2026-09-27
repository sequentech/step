// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {IvrExpectedInput} from "./IvrCall"

/** What a keypad hint's wording fills in; a plain record, as i18n libraries take. */
export type KeypadHintValues = {
    maxDigits: number
    /** The keys the prompt accepts, or `""` when any digits will do. */
    validInputs: string
    timeout: number
}

/** A keypad hint's two wordings, in whatever words and language the host uses. */
export interface KeypadHintWording {
    /** The prompt names the keys it accepts. */
    listed: (values: KeypadHintValues) => string
    /** Any digits will do, up to `maxDigits`, such as for a PIN. */
    anyKeys: (values: KeypadHintValues) => string
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
    const values = {maxDigits: expected.max_digits, validInputs, timeout: expected.timeout}
    return validInputs ? wording.listed(values) : wording.anyKeys(values)
}
