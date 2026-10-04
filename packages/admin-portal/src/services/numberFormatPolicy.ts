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
    id: string
    name: string
}

const sample = (policy: string): string => formatNumber(NUMBER_FORMAT_SAMPLE, policy, 2)

/** The policy an event's presentation names, if it is one this version doesn't know. */
const getUnknownNumberFormatPolicy = (presentation: unknown): string | undefined => {
    const policy: unknown =
        parseEntityPresentation<IElectionEventPresentation>(presentation)?.number_format_policy
    if (typeof policy !== "string" || policy === "") {
        return undefined
    }
    return resolveNumberFormatPolicy(policy) === policy ? undefined : policy
}

/**
 * One choice per policy, labelled like macOS's Number format menu: with a
 * sample. A policy the event names that this version doesn't know, such as a
 * newer version's, is listed too, so the event keeps it until another one is
 * chosen. Its label says it is shown as the default.
 */
export const getNumberFormatPolicyChoices = (
    presentation: unknown,
    t: (key: string, options: {policy: string; sample: string}) => string
): INumberFormatPolicyChoice[] => {
    const choices: INumberFormatPolicyChoice[] = Object.values(ENumberFormatPolicy).map(
        (policy) => ({id: policy, name: sample(policy)})
    )
    const unknown = getUnknownNumberFormatPolicy(presentation)
    return unknown === undefined
        ? choices
        : [
              ...choices,
              {
                  id: unknown,
                  name: t("electionEventScreen.field.numberFormatPolicy.unknownPolicy", {
                      policy: unknown,
                      sample: sample(unknown),
                  }),
              },
          ]
}

/**
 * The policy an election event writes its numbers with. Events whose
 * presentation names none, or one this version does not know, use the default.
 */
export const getElectionEventNumberFormatPolicy = (presentation: unknown): ENumberFormatPolicy =>
    resolveNumberFormatPolicy(
        parseEntityPresentation<IElectionEventPresentation>(presentation)?.number_format_policy
    )
