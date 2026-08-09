// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useMemo} from "react"
import {useAtomValue} from "jotai"
import {IElectionPresentation} from "@sequentech/ui-core"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {GetTallyDataQuery} from "@/gql/graphql"

const parseElectionPresentation = (presentation: unknown): IElectionPresentation | undefined => {
    if (!presentation) return undefined

    try {
        return typeof presentation === "string"
            ? (JSON.parse(presentation) as IElectionPresentation)
            : (presentation as IElectionPresentation)
    } catch {
        return undefined
    }
}

export function getDefaultElectionLang(
    tallyData: GetTallyDataQuery | null,
    electionId: string,
    electionEventId: string
): string | undefined {
    const election = tallyData?.sequent_backend_election?.find(
        (item) => item.id === electionId && item.election_event_id === electionEventId
    )

    return parseElectionPresentation(election?.presentation)?.language_conf?.default_language_code
}

export function useDefaultElectionLang(
    electionId: string,
    electionEventId: string
): string | undefined {
    const tallyData = useAtomValue(tallyQueryData)

    return useMemo(
        () => getDefaultElectionLang(tallyData, electionId, electionEventId),
        [tallyData?.sequent_backend_election, electionId, electionEventId]
    )
}
