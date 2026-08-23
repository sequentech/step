// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Turns the `message_map` of an `IInvalidPlaintextError` into options for
 * `t()`.
 *
 * Two things stand between the WASM boundary and a correctly pluralised
 * sentence:
 *
 * 1. `serde_wasm_bindgen` serialises a Rust `HashMap` as a JS `Map`, not as a
 *    plain object, so the map cannot be handed to `t()` as-is.
 * 2. Every value is a Rust `String`, and i18next skips plural handling
 *    altogether when `count` is a string. Numeric values have to be coerced.
 *
 * On top of that, the checker reports the *state* of the selection
 * (`numSelected`, `min`, `max`) while the voter needs the *remaining action*
 * ("select 2 more"). The number the sentence is about is therefore derived per
 * message key, and it is always emitted as a number so that a plural form
 * always resolves.
 */

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

export type BallotErrorOptions = Record<string, string | number>

/**
 * Accepts the shapes a `message_map` can arrive in — a JS `Map`, an array of
 * `[key, value]` tuples, or a plain object — and returns a plain object.
 */
const toRecord = (messageMap: unknown): Record<string, unknown> => {
    if (!messageMap) {
        return {}
    }

    if (messageMap instanceof Map) {
        return Object.fromEntries(messageMap)
    }

    if (Array.isArray(messageMap)) {
        const isEntryTupleArray = messageMap.every(
            (entry): entry is [string, unknown] =>
                Array.isArray(entry) && entry.length === 2 && typeof entry[0] === "string"
        )
        return isEntryTupleArray ? Object.fromEntries(messageMap) : {}
    }

    return isRecord(messageMap) ? messageMap : {}
}

/** `"3"` becomes `3`; anything that is not a number stays a string. */
const coerceValue = (value: unknown): string | number => {
    if (typeof value === "number") {
        return value
    }
    if (typeof value === "string" && value.trim() !== "") {
        const asNumber = Number(value)
        if (Number.isFinite(asNumber)) {
            return asNumber
        }
    }
    return String(value)
}

const asNumber = (value: string | number | undefined): number => {
    const coerced = typeof value === "number" ? value : Number(value)
    return Number.isFinite(coerced) ? coerced : 0
}

/**
 * The number each message is about, expressed as what the voter still has to
 * do rather than as the state the checker observed. A malformed map yields `0`
 * rather than `undefined`: i18next falls back to the unsuffixed key when
 * `count` is missing, and these keys only define `_one`/`_other`.
 */
const COUNT_DERIVATIONS: Record<string, (values: BallotErrorOptions) => number> = {
    // How many are still missing to reach the required minimum.
    "errors.implicit.selectedMin": (values) => asNumber(values.min) - asNumber(values.numSelected),
    // How many more may still be selected before reaching the maximum.
    "errors.implicit.underVote": (values) => asNumber(values.max) - asNumber(values.numSelected),
    // How many have to be deselected to get back to the maximum.
    "errors.implicit.selectedMax": (values) => asNumber(values.numSelected) - asNumber(values.max),
    "errors.implicit.maxSelectionsPerType": (values) =>
        asNumber(values.numSelected) - asNumber(values.max),
    // The maximum itself: the selection is already at it.
    "errors.implicit.overVoteDisabled": (values) =>
        values.max === undefined ? asNumber(values.numSelected) : asNumber(values.max),
}

export const getBallotErrorOptions = (
    message: string | null | undefined,
    messageMap: unknown
): BallotErrorOptions => {
    const options: BallotErrorOptions = {}
    for (const [key, value] of Object.entries(toRecord(messageMap))) {
        options[key] = coerceValue(value)
    }

    const derive = message ? COUNT_DERIVATIONS[message] : undefined
    if (derive) {
        options.count = Math.max(0, derive(options))
    }

    return options
}
