// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

type Option = {
    id: string
    name: string
}

type CandidateTypes = Option[]

export const CANDIDATE_TYPES = (t: any) => [
    {id: "candidate", name: t("candidateScreen.options.candidate")},
    {id: "option", name: t("candidateScreen.options.option")},
    {id: "write-in", name: t("candidateScreen.options.write-in")},
    {id: "open-list", name: t("candidateScreen.options.open-list")},
    {id: "closed-list", name: t("candidateScreen.options.closed-list")},
    {id: "semi-open-list", name: t("candidateScreen.options.semi-open-list")},
    {id: "invalid-vote", name: t("candidateScreen.options.invalid-vote")},
    {id: "blank-vote", name: t("candidateScreen.options.blank-vote")},
]
