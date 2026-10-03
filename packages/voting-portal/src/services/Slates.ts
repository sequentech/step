// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ICandidate, IContest} from "@sequentech/ui-core"

export interface ISlateContest {
    contest: IContest
    candidates: ICandidate[]
}

export interface IResolvedSlate {
    id: string
    name: Record<string, string>
    contests: ISlateContest[]
}

export const getSlateName = (
    slate: IResolvedSlate,
    language: string,
    defaultLanguage?: string
): string =>
    slate.name[language] ??
    (defaultLanguage ? slate.name[defaultLanguage] : undefined) ??
    Object.values(slate.name)[0] ??
    slate.id
