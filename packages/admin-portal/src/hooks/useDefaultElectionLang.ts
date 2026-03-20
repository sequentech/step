// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useMemo} from "react"
import {useAtomValue} from "jotai"
import {tallyQueryData} from "@/atoms/tally-candidates"

export function useDefaultElectionLang(electionId: string): string | undefined {
    const tallyData = useAtomValue(tallyQueryData)

    return useMemo(() => {
        const election = tallyData?.sequent_backend_election?.find((e) => e.id === electionId)
        if (!election?.presentation) return undefined
        try {
            return JSON.parse(election.presentation)?.language_conf
                ?.default_language_code as string | undefined
        } catch {
            return undefined
        }
    }, [tallyData?.sequent_backend_election, electionId])
}
