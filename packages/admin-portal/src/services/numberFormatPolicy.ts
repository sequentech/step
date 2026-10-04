// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ENumberFormatPolicy, formatNumber} from "@sequentech/ui-core"

const NUMBER_FORMAT_SAMPLE = 1234567.89

interface INumberFormatPolicyChoice {
    id: ENumberFormatPolicy
    name: string
}

/** One choice per policy, labelled like macOS's Number format menu: with a sample. */
export const getNumberFormatPolicyChoices = (): INumberFormatPolicyChoice[] =>
    Object.values(ENumberFormatPolicy).map((policy) => ({
        id: policy,
        name: formatNumber(NUMBER_FORMAT_SAMPLE, policy, 2),
    }))
