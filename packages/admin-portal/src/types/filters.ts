// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

type I18nString = {
    [key: string]: string
}

type Label = {
    i18n: I18nString
    name: string
}

// This is the key change - making it accept any operator
type FilterOperator = {
    [operator: string]: string | number | boolean | null
}

type FilterValue = string | number | boolean | FilterOperator

type Filter = {
    [key: string]: FilterValue
}

export type CustomFilter = {
    label: Label
    filter: Filter | null
}
