// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useMemo} from "react"
import {useAtomValue} from "jotai"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {GetTallyDataQuery} from "@/gql/graphql"
import {IElectionEventPresentation, IElectionPresentation} from "@sequentech/ui-core"

function parsePresentation<T>(presentation: unknown): T | undefined {
    if (!presentation) {
        return undefined
    }

    if (typeof presentation === "string") {
        try {
            return JSON.parse(presentation) as T
        } catch {
            return undefined
        }
    }

    if (typeof presentation === "object") {
        return presentation as T
    }

    return undefined
}

export function getDefaultElectionLang(
    tallyData: GetTallyDataQuery | null,
    electionId: string,
    electionEventId: string
): string | undefined {
    const election = tallyData?.sequent_backend_election?.find(
        (e) => e.id === electionId && e.election_event_id === electionEventId
    )
    const electionEvent = tallyData?.sequent_backend_election_event?.find(
        (ee) => ee.id === electionEventId
    )
    const eventPresentation = parsePresentation<IElectionEventPresentation>(electionEvent?.presentation)
    const electionPresentation = parsePresentation<IElectionPresentation>(election?.presentation)

    return (
        electionPresentation?.language_conf?.default_language_code ||
        eventPresentation?.language_conf?.default_language_code
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
