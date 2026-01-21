// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {atom} from "jotai"
import {GetTallyDataQuery} from "@/gql/graphql"

export const tallyQueryData = atom<GetTallyDataQuery | null>(null)
