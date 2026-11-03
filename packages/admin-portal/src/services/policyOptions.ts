// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export interface IPolicyOption<T extends string> {
    id: T
    name: string
}

/**
 * The choices of a select input for a policy: one per value, labelled with the
 * translation of `<labelKeyPrefix>.<value>`.
 */
export const policyOptions = <T extends string>(
    values: T[],
    labelKeyPrefix: string,
    translate: (key: string) => string
): IPolicyOption<T>[] =>
    values.map((value) => ({
        id: value,
        name: translate(`${labelKeyPrefix}.${value}`),
    }))
