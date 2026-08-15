// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useCallback, useEffect, useMemo} from "react"
import {useParams} from "react-router"
import {useAppSelector} from "../store/hooks"
import {IElectionExtended, selectElectionById} from "../store/elections/electionsSlice"
import {selectElectionEventById} from "../store/electionEvents/electionEventsSlice"
import {useTranslation} from "react-i18next"
import {ROOT_CLASS_PREFIX, toValidClassName, translateFromPresentation} from "@sequentech/ui-core"

const resolveElectionName = (
    election: IElectionExtended,
    language: string,
    eventDefaultLanguageCode?: string
) => {
    const presentation = election.presentation
    const defaultLanguageCode =
        presentation?.language_conf?.default_language_code ?? eventDefaultLanguageCode

    return (
        translateFromPresentation(presentation, "alias", language) ||
        translateFromPresentation(presentation, "name", language) ||
        (defaultLanguageCode
            ? translateFromPresentation(presentation, "alias", defaultLanguageCode) ||
              translateFromPresentation(presentation, "name", defaultLanguageCode)
            : undefined) ||
        election.alias ||
        election.name ||
        election.id
    )
}

/**
 * Manages election class on <html> and provides an election class formatter.
 */
export const useElectionClassName = () => {
    const {i18n} = useTranslation()
    const {electionId, eventId} = useParams<{electionId?: string; eventId?: string}>()
    const election = useAppSelector(selectElectionById(String(electionId ?? "")))
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const language = i18n.resolvedLanguage || i18n.language
    const eventDefaultLanguageCode =
        electionEvent?.presentation?.language_conf?.default_language_code

    const getElectionClassName = useCallback(
        (e: IElectionExtended) =>
            toValidClassName(resolveElectionName(e, language, eventDefaultLanguageCode)),
        [eventDefaultLanguageCode, language]
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
