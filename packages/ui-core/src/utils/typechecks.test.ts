// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {expect, it} from "@jest/globals"
import {isNumber, isString, isArray, isNull, isUndefined, isObject} from "./typechecks"

it("narrows primitive and array values without coercing strings or empty values", () => {
    expect(isNumber(0)).toBe(true)
    expect(isNumber("0")).toBe(false)
    expect(isString("")).toBe(true)
    expect(isString(null)).toBe(false)
    expect(isArray([])).toBe(true)
    expect(isArray({length: 0})).toBe(false)
    expect(isNull(null)).toBe(true)
    expect(isNull(undefined)).toBe(false)
    expect(isUndefined(undefined)).toBe(true)
    expect(isUndefined(null)).toBe(false)
})

it("does not narrow null to an object that consumers may dereference", () => {
    expect(isObject({})).toBe(true)
    expect(isObject([])).toBe(true)
    expect(isObject("text")).toBe(false)
    expect(isObject(null)).toBe(false)
})
