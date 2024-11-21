// SPDX-FileCopyrightText: 2022-2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const isNumber = (value: any): value is number => "number" === typeof value

export const isString = (value: any): value is string => "string" === typeof value

export const isArray = (value: any): boolean => Array.isArray(value)

export const isNull = (value: any): value is null => null === value

export const isUndefined = (value: any): value is undefined => undefined === value

export const isObject = (value: any): value is object => typeof value === "object"
