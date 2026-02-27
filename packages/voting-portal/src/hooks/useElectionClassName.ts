// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useCallback, useEffect, useMemo} from "react"
import {useParams} from "react-router"
import {useAppSelector} from "../store/hooks"
import {IElectionExtended, selectElectionById} from "../store/elections/electionsSlice"
import {useTranslation} from "react-i18next"
import {ROOT_CLASS_PREFIX, toValidClassName, translateElection} from "@sequentech/ui-core"

/**
 * Manages election class on <html> and provides an election class formatter.
 */
export const useElectionClassName = () => {
    const {i18n} = useTranslation()
    const {electionId} = useParams<{electionId?: string}>()
    const election = useAppSelector(selectElectionById(String(electionId ?? "")))

    const extractElectionName = (election: IElectionExtended) => {
        let language = i18n.resolvedLanguage || i18n.language
        return (
            translateElection(election, "alias", language) ||
            translateElection(election, "name", language) ||
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
        const appRoot = document.querySelector(".app-root")

        if (!appRoot) return

        // Remove any previous e-* class we added
        for (const cls of Array.from(appRoot.classList)) {
            if (cls.startsWith(ROOT_CLASS_PREFIX)) {
                appRoot.classList.remove(cls)
            }
        }

        if (activeElectionClassName) {
            appRoot.classList.add(activeElectionClassName)
        }
    }, [activeElectionClassName])

    return [getElectionClassName, activeElectionClassName] as const
}
