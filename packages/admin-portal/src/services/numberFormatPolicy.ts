// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    ENumberFormatPolicy,
    formatNumber,
    IElectionEventPresentation,
    parseEntityPresentation,
    resolveNumberFormatPolicy,
} from "@sequentech/ui-core"

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

/**
 * The policy an election event writes its numbers with. Events whose
 * presentation names none, or one this version does not know, use the default.
 */
export const getElectionEventNumberFormatPolicy = (presentation: unknown): ENumberFormatPolicy =>
    resolveNumberFormatPolicy(
        parseEntityPresentation<IElectionEventPresentation>(presentation)?.number_format_policy
    )
