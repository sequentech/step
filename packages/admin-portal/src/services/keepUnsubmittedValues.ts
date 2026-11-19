// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const isPlainObject = (value: unknown): value is Record<string, unknown> =>
    value !== null &&
    typeof value === "object" &&
    !Array.isArray(value) &&
    Object.getPrototypeOf(value) === Object.prototype

/**
 * `submitted` with every key it lacks taken from `kept`, at any depth.
 *
 * react-hook-form leaves the values of disabled inputs out of what a form
 * submits (it unsets them, and an object left empty with them), although the
 * form still holds them. A save that writes a whole JSON column from the
 * submitted values would then drop them. Only missing keys are filled: a key
 * the form submits, even empty, null or undefined (a deliberate clear), wins;
 * arrays and other values are taken whole, never merged.
 */
export function keepUnsubmittedValues<T>(submitted: T, kept: unknown): T {
    if (!isPlainObject(submitted) || !isPlainObject(kept)) return submitted
    const result: Record<string, unknown> = {...submitted}
    for (const [key, value] of Object.entries(kept)) {
        result[key] = Object.prototype.hasOwnProperty.call(submitted, key)
            ? keepUnsubmittedValues(submitted[key], value)
            : value
    }
    return result as T
}
