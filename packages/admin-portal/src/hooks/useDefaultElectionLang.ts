// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useMemo} from "react"
import {useAtomValue} from "jotai"
import {IElectionEventPresentation, IElectionPresentation} from "@sequentech/ui-core"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {GetTallyDataQuery} from "@/gql/graphql"

const parsePresentation = <T,>(presentation: unknown): T | undefined => {
    if (!presentation) return undefined

    try {
        return typeof presentation === "string"
            ? (JSON.parse(presentation) as T)
            : (presentation as T)
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
    const electionEvent = tallyData?.sequent_backend_election_event?.[0]

    return (
        parsePresentation<IElectionPresentation>(election?.presentation)?.language_conf
            ?.default_language_code ||
        parsePresentation<IElectionEventPresentation>(electionEvent?.presentation)?.language_conf
            ?.default_language_code
    )
}

export function useDefaultElectionLang(
    electionId: string,
    electionEventId: string
): string | undefined {
    const tallyData = useAtomValue(tallyQueryData)

    return useMemo(
        () => getDefaultElectionLang(tallyData, electionId, electionEventId),
        [
            tallyData?.sequent_backend_election,
            tallyData?.sequent_backend_election_event,
            electionId,
            electionEventId,
        ]
    )
}
