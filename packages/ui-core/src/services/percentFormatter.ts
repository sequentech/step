// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {formatNumber} from "./numberFormat"

export const formatPercentOne = (percentOne: number, policy?: string | null) =>
    `${numberToNDecimalPlaces(percentOne * 100, 2, policy)}%`

export const numberToNDecimalPlaces = (
    num: number,
    decimals: number,
    policy?: string | null
): string => formatNumber(num, policy, decimals)
