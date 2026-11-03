// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {BallotSelection} from "@sequentech/ui-core"
import {IResolvedSlate} from "./Slates"

export type SlateMembers = Record<string, string[]>

export const getSlateMembers = (slate: IResolvedSlate): SlateMembers =>
    Object.fromEntries(
        slate.contests.map(({contest, candidates}) => [
            contest.id,
            candidates.map((candidate) => candidate.id),
        ])
    )

export enum ESlateSelectionStatus {
    NONE = "none",
    PARTLY = "partly",
    MIXED = "mixed",
    ALL = "all",
}

export interface ISlateSelectionSummary {
    status: ESlateSelectionStatus
    selected: number
    total: number
    selectedMemberIds: string[]
}

/**
 * How much of a slate the ballot currently holds, read from the ordinary
 * candidate selections. No slate choice is stored anywhere: a member chosen
 * one by one counts exactly like one chosen through the slate.
 *
 * The whole ballot is looked at: a slate is ALL only when the ballot holds
 * exactly its candidates and nothing in the offices it has none for. Any
 * other choice next to its members makes it MIXED.
 */
export const getSlateSelectionSummary = (
    members: SlateMembers,
    selection: BallotSelection | undefined
): ISlateSelectionSummary => {
    const selectedMemberIds: string[] = []
    let total = 0
    let hasOutsideChoice = false

    for (const contest of selection ?? []) {
        const memberIds = members[contest.contest_id] ?? []
        if (contest.is_explicit_invalid) {
            hasOutsideChoice = true
        }
        for (const choice of contest.choices) {
            const isMember = memberIds.includes(choice.id)
            const isSelected = choice.selected > -1
            if (isMember) {
                total += 1
                if (isSelected) {
                    selectedMemberIds.push(choice.id)
                }
            } else if (isSelected) {
                hasOutsideChoice = true
            }
        }
    }

    const selected = selectedMemberIds.length
    let status = ESlateSelectionStatus.PARTLY
    if (selected === 0) {
        status = ESlateSelectionStatus.NONE
    } else if (hasOutsideChoice) {
        status = ESlateSelectionStatus.MIXED
    } else if (selected === total) {
        status = ESlateSelectionStatus.ALL
    }

    return {status, selected, total, selectedMemberIds}
}
