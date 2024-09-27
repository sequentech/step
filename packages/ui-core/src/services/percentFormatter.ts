// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const formatPercentOne = (percentOne: number) =>
    `${numberToNDecimalPlaces(percentOne * 100, 2)}%`

export const numberToNDecimalPlaces = (num: number, decimals: number): string =>
    new Intl.NumberFormat("en-US", {
        minimumFractionDigits: decimals,
        maximumFractionDigits: decimals,
    }).format(num)
