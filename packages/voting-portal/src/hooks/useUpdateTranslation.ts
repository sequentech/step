// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect} from "react"
import {IElectionEvent} from "../store/electionEvents/electionEventsSlice"
import {ETranslationScope, overwriteTranslations} from "@sequentech/ui-core"

type props = {
    electionEvent: IElectionEvent | undefined
}
const useUpdateTranslation = (
    {electionEvent}: props,
    defaultLanguageTouched: boolean,
    setDefaultLanguageTouched: (value: boolean) => void
) => {
    // Overwrites translations based on the election event presentation
    // Update Language based on presentation only if default language has not been touched,
    // So search param "lang" > user selected locale (saved in cookie) >
    // language detection policy (force default) > browser settings
    useEffect(() => {
        const hasSetDefaultLanguage = overwriteTranslations(electionEvent?.presentation, {
            scope: ETranslationScope.VOTING_PORTAL,
            legacyScope: ETranslationScope.VOTING_PORTAL,
            changeDefaultLanguage: !defaultLanguageTouched,
        })
        if (hasSetDefaultLanguage) {
            setDefaultLanguageTouched(true)
        }

        return () => {
            overwriteTranslations(undefined, {
                scope: ETranslationScope.VOTING_PORTAL,
                legacyScope: ETranslationScope.VOTING_PORTAL,
                changeDefaultLanguage: false,
            })
        }
    }, [electionEvent?.presentation])

    return {}
}

export default useUpdateTranslation
