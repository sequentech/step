// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useCallback, useEffect, useMemo} from "react"
import {useParams} from "react-router"
import {useAppSelector} from "../store/hooks"
import {IElectionExtended, selectElectionById} from "../store/elections/electionsSlice"
import {useTranslation} from "react-i18next"
import {translateElection} from "@sequentech/ui-core"

const MAX_CLASSNAME_LENGTH = 40
const ROOT_CLASS_PREFIX = "e-"

function toValidClassName(input?: string): string | null {
    if (!input) return null

    let result = input.replace(/\s+/g, "")
    result = result.replace(/[^a-zA-Z0-9-_]/g, "")

    result = `${ROOT_CLASS_PREFIX}${result}`
    result = result.slice(0, MAX_CLASSNAME_LENGTH)

    return result.length ? result : null
}

/**
 * Manages election class on <html> and provides an election class formatter.
 */
export const useElectionClassName = () => {
    const {i18n} = useTranslation()
    const {electionId} = useParams<{electionId?: string}>()
    const election = useAppSelector(selectElectionById(String(electionId ?? "")))

    const extractElectionName = (election: IElectionExtended) => {
        return (
            translateElection(election, "alias", i18n.language) ||
            translateElection(election, "name", i18n.language) ||
            election.alias ||
            election.name ||
            election.id
        )
    }

    const getElectionClassName = useCallback(
        (e: IElectionExtended) => toValidClassName(extractElectionName(e)),
        [extractElectionName]
    )

    const activeElectionClassName = useMemo(() => {
        if (!electionId || !election) return null
        return getElectionClassName(election)
    }, [electionId, election, getElectionClassName])

    useEffect(() => {
        const root = document.documentElement

        // Remove any previous e-* class we added
        for (const cls of Array.from(root.classList)) {
            if (cls.startsWith(ROOT_CLASS_PREFIX)) {
                root.classList.remove(cls)
            }
        }

        if (activeElectionClassName) {
            root.classList.add(activeElectionClassName)
        }
    }, [activeElectionClassName])

    return [getElectionClassName, activeElectionClassName] as const
}
