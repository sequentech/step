// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const isNumber = (value: unknown): value is number => "number" === typeof value

export const isString = (value: unknown): value is string => "string" === typeof value

export const isArray = (value: unknown): value is unknown[] => Array.isArray(value)

export const isNull = (value: unknown): value is null => null === value

export const isUndefined = (value: unknown): value is undefined => undefined === value

export const isObject = (value: unknown): value is object =>
    value !== null && typeof value === "object"
