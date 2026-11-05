// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useParams} from "react-router-dom"
import {useTranslation} from "react-i18next"
import {translateFromPresentation} from "@sequentech/ui-core"
import {TenantEventType} from ".."
import {selectElectionEventById} from "../store/electionEvents/electionEventsSlice"
import {selectElectionById} from "../store/elections/electionsSlice"
import {useAppSelector} from "../store/hooks"

/** The election's name in the voter's language, when the portal holds it. */
export const useElectionName = (electionId: string | undefined): string | undefined => {
    const {i18n} = useTranslation()
    const {eventId} = useParams<TenantEventType>()
    const election = useAppSelector(selectElectionById(String(electionId)))
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    if (!election) return undefined
    const defaultLanguageCode =
        election.presentation?.language_conf?.default_language_code ??
        electionEvent?.presentation?.language_conf?.default_language_code
    return (
        translateFromPresentation(election, "name", i18n.language, {defaultLanguageCode}) ||
        undefined
    )
}
