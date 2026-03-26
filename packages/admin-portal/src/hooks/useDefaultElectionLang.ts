// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useMemo} from "react"
import {useAtomValue} from "jotai"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {GetTallyDataQuery} from "@/gql/graphql"
import {IElectionEventPresentation, IElectionPresentation} from "@sequentech/ui-core"

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
    try {
        let eventDefaultLang: string | undefined
        let electionDefaultLang: string | undefined
        if (electionEvent?.presentation) {
            eventDefaultLang = (
                JSON.parse(electionEvent.presentation) as IElectionEventPresentation
            )?.language_conf?.default_language_code
        }

        if (election?.presentation) {
            electionDefaultLang = (JSON.parse(election.presentation) as IElectionPresentation)
                ?.language_conf?.default_language_code
        }
        return electionDefaultLang || (eventDefaultLang as string | undefined)
    } catch {
        return undefined
    }
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
